# Open timing

How long the window takes to open, step by step, from the moment it's asked for (a keybind's
`--toggle`) to the thumbnails on screen.

    cargo tauri build --no-bundle --features automation
    scripts/open-timing/run.sh [OPENS]

The app records `performance.mark`s as it starts (`ui/src/perf.rs`: UI code running, drawn, each
start-up request back, library scanned, tiles created and painted, thumbnails); `collect.js`, run
in the window by the automation build, reports them to the app, which prints them with when the
window was asked for and built. The marks cost nothing in normal builds; nothing reads them.

First measurements (2026-10-06, headless, opens after the first): ~600 ms to the thumbnails
before, ~500 ms after sending the start-up requests together and giving the grid the cached
thumbnails' paths with the scan.
