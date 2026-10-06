# Local tab in GPUI (prototype)

The Local tab rebuilt in [GPUI](https://gpui.rs) with gpui-kit's components, to compare with the
WebKit app before deciding on a port. Same core crate: library scan, thumbnails, favorites and
tags, applying. Only the grid, search, Favorites and click-to-apply (on the focused monitor).

    cd prototype/gpui-local
    cargo run --release

The tiles copy the app's (`ui/src/views/local/tile.rs`): card, inset picture, monitor badges,
name and size, tags, the star (it toggles the favorite). GPUI has no style transitions, so the
hover lift and glow switch at once, and no zoom on the picture. A mouse wheel glides (GPUI
jumps a step per notch); touchpads scroll as they come.

`fonts/`: fixed weights of the app's Nunito (a variable font, whose default weight is all GPUI
uses), cut with `fonttools varLib.instancer Nunito.ttf wght=700`.

Its own workspace and `target/`: the app's build doesn't change. The first build takes ~4 min.

`WALLPICKER_AUTOSCROLL=wheel` sends 60 wheel notches through the real wheel handler (it caught a
panic headless). `WALLPICKER_AUTOSCROLL=1` scrolls through the whole library and back (for measuring against
the WebKit app with the same workload).

First measurement (headless, the same scroll through 135 wallpapers): 94 MB when opened and
160 MB after scrolling everything, in one process; the WebKit app: 341 and 358 MB in five.
