# Local tab in Quickshell (prototype)

The Local tab rebuilt in [Quickshell](https://quickshell.org) (QML), as 1:1 with the app as it
goes, to judge it against the WebKit app before deciding on a port. The Rust side is unchanged:
everything goes through the CLI (`Backend.qml`: `wallpicker list --json --thumbs`, `apply`,
`favorite`, `previous`; `Theme.qml`: `wallpicker theme --json`, mapped onto the same tokens as
`ui/src/palette.rs`).

    qs -p prototype/quickshell-local
    qs -p prototype/quickshell-local ipc call wallpicker toggle    # close / open (a keybind)

Works: the top bar (tabs as looks only, monitor pills and All choose where a click applies,
Previous), the search card (search, Favorites, sort, Random, tags; Color, Flat and ⋯ are looks
only), the tiles (click applies, the star toggles the favorite), pages of 60 with Jump to.

`fonts/`: fixed weights cut from the app's Nunito (a variable font; Qt only uses its default,
light weight), inside the config folder because Quickshell loads nothing from outside it.

For measuring: `WP_NOIMG=1` (no thumbnails), `WP_NOFX=1` (no blur, glass or shadows); the log
says when the first screen of thumbnails is in after an open.

## Measured (2026-10-06, headless, 1920×1080, your library)

|                                         | WebKit app        | This prototype                   |
|-----------------------------------------|-------------------|----------------------------------|
| Open, Local tab                         | ~300-360 MB       | ~215-260 MB (one process)        |
| Open to the first screen of thumbnails  | ~0.5-0.6 s (from the tray) | ~0.16 s (window rebuilt), ~0.43 s (from nothing) |
| Between opens                           | ~110-125 MB (tray) | ~200-230 MB if Quickshell stays; 0 if it exits |

Where the memory goes: Quickshell alone 75 MB, with an empty window 135 MB (Qt Quick and the GPU),
our UI ~80 MB more, of which thumbnails ~14 MB; blur and shadows cost nothing measurable.
Destroying the window gives back only ~40-50 MB (and it creeps ~10 MB a cycle; Qt's image cache
off and glibc trimming don't change it), so the light way is starting Quickshell per open, which
is still faster than the app's reopen.
