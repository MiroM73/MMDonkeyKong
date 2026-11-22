MMDonkeyKong - offline web bundle
=================================

This folder is the browser version of the game. Browsers do not run WebAssembly from file://,
so serve the folder with any static web server and open the address it prints:

    python -m http.server 8080          (then open http://localhost:8080/)

or: npx serve . / any other static server. After the first visit the game also works without
the server's network (installable web app); the Windows exe is the real offline version.

Online: https://mirom73.github.io/MMDonkeyKong/
Source: https://github.com/MiroM73/MMDonkeyKong
License: the Unlicense (LICENSE).
