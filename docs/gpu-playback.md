# GPU playback primitives

Implemented on 2026-10-11 in `crates/wallpicker/src/renderer/gpu/playback/`.
The daemon uses it behind the experimental `animation.enabled` setting. The renderer's playback
and transition path connects these primitives to prepared sessions, tagged callbacks and deadline timers.
No animation surface or texture is created during a static-only session.

## Ownership and uploads

- `Texture` owns one source-sized RGBA allocation and its logical byte reservation. Outputs
  share it with `Rc`, so the same source frame uploads once per session. `upload()` validates
  generation, sequence, dimensions and exact RGBA byte length before submission. Repeated
  sequence numbers are skipped; older sequences/generations are rejected. Updates use
  `tex_sub_image_2d` without resizing, conversion, readback or an intermediate CPU image.
- `Painter` owns the shared program/quad and cached uniform locations. It draws the source
  texture with per-output geometry. Drawing allocates no texture or vertex buffer and does
  not upload pixels, request callbacks or advance playback.
- `Surface` owns a persistent EGL window surface and native `wl_egl_window`. Resizing uses
  `wl_egl_window_resize`; it retains the EGL surface and source texture. Its owner supplies a
  configured Wayland surface, applies viewport/buffer scale, schedules draws and destroys
  the GPU surface before its layer role. Swap interval is zero to avoid a second vsync wait
  on top of the frame-callback/deadline scheduler.

Objects stay on the renderer thread. The EGL device is shared with transition rendering;
playback and transitions establish their own GL state when drawing. Each GL resource is
deleted with its owning context current, and the EGL surface is destroyed before its native
window. A texture reservation remains until its final owner disappears.

The texture budget charges `width × height × 4` once per source. Dimensions must satisfy
the 16,384-axis input guard and actual GLES texture/viewport limits. This accounts for logical
source storage, not driver staging, EGL swapchain buffers, retained transition content or
total GPU RSS. Those still need measurement and separate admission policy.
No shipping GPU budget or upload-rate default is selected by this implementation.

## Geometry and alpha

Mapping retains the existing transform order: source region or portable frame, then flips.
Regions and framed crops fill the output; their explicit view overrides the general fit.
Portable zoom/pan is resolved against the current output aspect using `Frame::rect`, with
tiny edge crops clamped inside the source. Full-image views support:

- Cover: centered crop at the output aspect.
- Contain: integer-sized centered image with the same black-bar placement as static compose.
- Fill: independent horizontal/vertical scaling.
- Tile: original-size repeats from the top left, including non-power-of-two sources.

Flips act within the chosen source view or each tile. Decoder rows remain top-first and the
shader corrects GLES texture orientation. Crop-edge sampling stays within the selected view.

Straight-alpha frames are sampled with nearest texture filtering and four shader samples.
Each sample is premultiplied before bilinear interpolation, then presented opaque over black.
This prevents hidden RGB in transparent pixels from bleeding into visible edges. Static
images continue using their existing Lanczos path; the GPU path preserves placement and
geometry, but its scaling filter is bilinear rather than Lanczos.

Incoming transitions call this same geometry sampler through the shared static-transition
effect masks. Mask coverage multiplies premultiplied RGBA, including opaque black bars;
Fade uses shader opacity. Explicit zero/one endpoints guarantee a transparent start and
the exact steady playback image at completion. Slide moves the mapped output image rather
than shifting source crop coordinates. Every effect shares the existing source texture and
program; transition draws do not upload or allocate additional image storage.

## Validation and remaining integration

Pure tests cover fits, crop/flip order, resize-dependent framing, tiny crops, invalid geometry
and reference-counted budget lifetime. The explicit EGL test creates headless pbuffers,
renders and reads them back **only in the test harness**, without using a compositor:

```sh
cargo test -p wallpicker --offline headless_gpu_pixels_reuse_and_cleanup -- --ignored --nocapture
```

On 2026-10-11 this passed on `AMD Radeon RX 9060 XT` with Mesa radeonsi. It verifies source
orientation, all fits, tiling/flips, regions, framing, transparent/partial-alpha edges,
texture reuse across different output sizes, shared uploads, rejection of stale/malformed
frames, budget refusal and final texture deletion. It also compiles the existing transition
shaders after the common shader-compiler refactor. On 2026-10-12 expanded pixel checks passed
transparent starts, exact endpoints, intermediate premultiplied coverage and slide positions
for every effect across fits, crops, framing and flips. The test permits software GLES for CI;
production still rejects software rasterizers.

That pixel test does not validate Wayland windows. On 2026-10-12 the separate
[private-compositor qualification](wayland-playback.md) passed actual window swaps, resize,
virtual output removal/re-addition, scaling/rotation and moving GIF playback on the same
hardware GPU. It also verifies shared upload counts, resource teardown and all incoming
transition effects, including frozen-frame uploads and endpoint handoff. Physical display
timing, pause/lock controls and driver/swapchain memory measurements remain open.
Hardware EGL initialization failure now selects a [representative still](representative-fallback.md);
texture-budget rejection continues to preserve old content and return an error.
Sharing a texture is not a frozen snapshot: the incoming source stays paused throughout its
transition. Outgoing content is frozen in the previously submitted surface buffer, allowing
unaffected outputs sharing its source to continue uploading and playing.
