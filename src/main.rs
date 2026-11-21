// In release builds on Windows, do not open a console window.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
// The audio / storage traits are not used by the front end yet (issues #11, #16); each allow is
// removed when its module is wired in.
#[allow(dead_code)]
mod audio;
mod config;
mod game;
mod i18n;
// The window icon is only needed by the desktop window (the browser tab uses `index.html`).
#[cfg(not(target_arch = "wasm32"))]
mod icon;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[allow(dead_code)]
mod storage;

/// Desktop entry point.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let (cfg, _) = config::Config::load();
    i18n::set_language(&cfg.language);
    native::start()
}

/// Browser entry point: starts the game on the canvas `game_canvas` of `index.html`.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    wasm_bindgen_futures::spawn_local(async {
        let document = eframe::web_sys::window()
            .and_then(|w| w.document())
            .expect("no browser document");
        let canvas = document
            .get_element_by_id("game_canvas")
            .expect("index.html has no canvas with the id game_canvas")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("game_canvas is not a canvas");
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
            )
            .await;
        // The loading text of index.html is replaced by the error, or removed on success.
        if let Some(el) = document.get_element_by_id("loading") {
            match result {
                Ok(()) => el.remove(),
                Err(e) => el.set_inner_html(&format!("Could not start the game: {e:?}")),
            }
        }
    });
}
