# Representative still fallback

Implemented in the qualified renderer path on 2026-10-12. If hardware EGL initialization
or playback-painter setup fails, the renderer shows the prepared representative frame and
reports playback as unavailable. Software GLES is still rejected. Texture-budget exhaustion,
decoder admission failure and malformed media remain errors; they do not silently select
fallback in the renderer. The daemon's policy catches them for a request, says why, and shows
that request's animations as first-frame stills (the same stills as with playback off).

## Ownership and presentation

GPU preflight leaves the prepared handoff with its caller and returns a fallback offer.
The caller converts each already decoded first frame to RGB over black, reserving its storage
before allocation. Equal source identities within the application share one RGB image.
Dropping each prepared session cancels and joins its decoder on the caller, away from the
Wayland loop. Every animated source alias is checked again before queuing fallback display.
There is no second decode or temporary image file.

Each output fits that RGB image on the ordinary image worker and submits an opaque SHM
buffer. Source regions are cropped before filtering, so neighboring source colors cannot
bleed into a standalone cut. Framing, fits, flips and alpha-over-black behavior are retained;
CPU fitting uses Lanczos3 rather than the GPU's bilinear filter. Fallback animation changes
are immediate. Static members retain the existing static transition behavior.

The shown selection retains the original animated source, file identity and application ID.
Resize uses the retained representative pixels even if the original file has disappeared.
Adjustment preflight still checks the selected file identity, then reuses the matching still
and application ID. A new application prepares a fresh representative frame. Replacement,
missing outputs, disconnect and shutdown release unreferenced RGB images. No playback
runtime, decoder or playback timer remains active in an EGL-unavailable session.

## Budgets and failure behavior

The explicit qualification option `fallback_bytes` covers retained RGB images and temporary
fitting reservations. Source storage charges `width × height × 3` once per shared image.
Fitting reserves twice the source RGB bytes plus four times the destination XRGB bytes before
starting pixel work; the reservation remains attached to the result until submission/drop.
Outputs are limited to 16,384 pixels per axis and 64 MiB XRGB. Invalid/oversized dimensions
and insufficient budgets fail before fitting allocations.

This is logical image-buffer accounting, not a hard process-RSS bound. Resize-library scratch,
allocator overhead, compositor buffers and the existing SHM buffer-release lifecycle still
need desktop-size measurements. The 128 MiB test allowance is not a shipping default.

Admission failure leaves every output unchanged. Per-output fitting/submission failure keeps
the old wallpaper and reports an error. A failed fallback resize retains the existing buffer
and waits for another request/output change, rather than repeatedly retrying a permanent
budget failure. Successful outputs commit independently during partial application.

Status reports `playback.state: "unavailable"`, the original EGL failure reason, representative
mode and active geometry. `representative_bytes` includes retained source images and pending
fitting reservations. `render_error` reports fitting/submission failures. In steady fallback
state, playback worker count, GPU texture bytes and playback-timer presence are all zero.

## Qualification

```sh
python3 scripts/test-playback-headless.py --fallback
python3 scripts/test-playback-headless.py --fallback --source "$HOME/Pictures/Wallpapers/775121.gif"
python3 scripts/test-playback-headless.py --fallback --source "$HOME/Pictures/Wallpapers/869910.gif"
python3 scripts/test-playback-headless.py --fallback --source "$HOME/Pictures/Wallpapers/1q3f7z5fv7hf1.gif"
```

The script starts private KWin virtual outputs, then gives only the test renderer
`LIBGL_ALWAYS_SOFTWARE=1`. The actual software-renderer rejection triggers SHM fallback.
Generated media and all three downloaded GIFs passed shared storage, decoder release,
adjustment, rotation, repeated replacement, partial failure, redraw after source removal,
static return, shutdown and admission/fitting failure preservation. Unit tests check alpha,
crop-edge filtering, geometry, overflow/size guards and reservation release.

The normal hardware-GPU lifecycle/transition suite also passed with a wallpaper-sized GIF
after this integration. These tests establish successful submissions and ownership behavior;
compositor screenshot comparisons and physical-output resource measurements remain open.
