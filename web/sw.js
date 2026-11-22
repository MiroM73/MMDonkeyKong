// Service worker of the installable web app (docs/platforms.md -> Offline use).
//
// Strategy: cache-first. Everything the page needs is stored on install, so the game starts with
// no network (and instantly on a weak one). A new build changes the bytes of this file (the CI
// replaces __BUILD__ with the commit, see .github/workflows/pages.yml), the browser installs the
// new worker next to the old one and the page shows an "update available" hint; the new worker
// takes over when the player taps it (message "skip-waiting"), the old cache is then deleted.
const BUILD = '__BUILD__';
const CACHE = 'mmdonkeykong-' + BUILD;
const SCOPE = self.registration.scope;
// Files that are not mentioned in index.html.
const EXTRA = ['manifest.webmanifest', 'icon-192.png', 'icon-512.png', 'icon-maskable-512.png', 'apple-touch-icon.png'];

// The files of this build: index.html plus every .js / .wasm / .css it refers to (Trunk puts
// hashed names there) plus EXTRA.
async function precacheUrls() {
  const res = await fetch(SCOPE, { cache: 'reload' });
  const html = await res.clone().text();
  const urls = new Set([SCOPE, ...EXTRA.map((p) => new URL(p, SCOPE).href)]);
  for (const m of html.matchAll(/["']([^"'\s]+\.(?:js|wasm|css))["']/g)) {
    const u = new URL(m[1], SCOPE);
    if (u.origin === self.location.origin) urls.add(u.href);
  }
  return { urls: [...urls], index: res };
}

self.addEventListener('install', (event) => {
  event.waitUntil((async () => {
    const { urls, index } = await precacheUrls();
    const cache = await caches.open(CACHE);
    await cache.put(SCOPE, index);
    await Promise.all(urls.filter((u) => u !== SCOPE).map(async (u) => {
      const res = await fetch(u, { cache: 'reload' });
      if (!res.ok) throw new Error('precache failed: ' + u);
      await cache.put(u, res);
    }));
  })());
});

self.addEventListener('message', (event) => {
  if (event.data === 'skip-waiting') self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    for (const key of await caches.keys()) {
      if (key.startsWith('mmdonkeykong-') && key !== CACHE) await caches.delete(key);
    }
    await self.clients.claim();
  })());
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET' || new URL(req.url).origin !== self.location.origin) return;
  event.respondWith((async () => {
    const cache = await caches.open(CACHE);
    // A page load (any URL inside the scope) gets the cached index.html.
    const hit = await cache.match(req.mode === 'navigate' ? SCOPE : req, { ignoreSearch: true });
    if (hit) return hit;
    try {
      const res = await fetch(req);
      if (res.ok) cache.put(req, res.clone());
      return res;
    } catch (err) {
      if (req.mode === 'navigate') return (await cache.match(SCOPE)) || Response.error();
      throw err;
    }
  })());
});
