# Local tab in GPUI (prototype)

The Local tab rebuilt in [GPUI](https://gpui.rs) with gpui-kit's components, to compare with the
WebKit app before deciding on a port. Same core crate: library scan, thumbnails, favorites and
tags, applying. Only the grid, search, Favorites and click-to-apply (on the focused monitor).

    cd prototype/gpui-local
    cargo run --release

Its own workspace and `target/`: the app's build doesn't change. The first build takes ~4 min.

`WALLPICKER_AUTOSCROLL=1` scrolls through the whole library and back (for measuring against
the WebKit app with the same workload).

First measurement (headless, the same scroll through 135 wallpapers): 94 MB when opened and
160 MB after scrolling everything, in one process; the WebKit app: 341 and 358 MB in five.
