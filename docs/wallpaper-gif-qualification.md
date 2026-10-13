# Wallpaper-sized GIF qualification

Measured on 2026-10-11 using three user-provided local GIFs in `~/Pictures`. No assets were
copied into the repository and no desktop wallpaper or saved application state was changed.
The machine uses an AMD Ryzen 5 5600X. The workspace release profile uses `opt-level = "s"`,
LTO and one codegen unit. These are decoder/IPC measurements, **not GPU playback benchmarks**.

## Reproduce

```sh
cargo run --release --offline -p wallpicker --example qualify_animation -- \
  --loops 10 --child-mib 128 \
  "$HOME/Pictures/Wallpapers/775121.gif" \
  "$HOME/Pictures/Wallpapers/869910.gif" \
  "$HOME/Pictures/Wallpapers/1q3f7z5fv7hf1.gif"
```

The example accepts other local animated media paths and prints one JSON report per input.
It runs its own private decoder helper under the same hard address-space limit, pull protocol
and three leased frame slots as the application worker. Its defaults (three measured loops,
128 MiB child address space) are test settings, not shipping playback policy.

Each source gets one complete warm-up loop and the requested measured loops. Every frame's
RGBA fingerprint and original delay must match the warm-up loop, including across decoder
restarts at loop boundaries. Only fingerprints and durations are retained, never a decoded
animation. RSS is sampled every 20 ms, with additional snapshots after warm-up, measurement,
three-slot backpressure and replacement. Throughput includes IPC, pixel hashing and loop
validation; it is neither pure codec speed nor paced on-screen playback.

After measurement, the test holds all three frame leases, cancels/reaps the worker, checks
that admission remains charged until the held frames are released, and verifies zero remaining
reservations. Three further first-frame preparations validate replacement consistency and
cleanup. A deliberately insufficient 1 MiB child limit was also checked: failure stayed in the
isolated child and the tool returned an error. Invalid loop counts are rejected before startup.

## Results

| Input | Dimensions | Encoded size | Frames/loop | Playback duration/loop | Original frame delays |
| --- | --- | ---: | ---: | ---: | --- |
| `775121.gif` | 1920×1080 | 0.47 MiB | 24 | 3.84 s | 160 ms |
| `869910.gif` | 1920×1080 | 1.70 MiB | 6 | 0.51 s | 0–100 ms |
| `1q3f7z5fv7hf1.gif` | 1925×1075 | 13.00 MiB | 259 | 28.40 s | 20–110 ms |

`869910.gif` encodes a 0.50 s loop. The provisional 10 ms minimum increases its playback
duration to 0.51 s; encoded timing is preserved separately.

| Input | First frame | Measured frames/s, including validation | Source-time/wall-time ratio | Sampled child peak RSS | Parent RSS, all three frames held | Backpressured cancellation |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `775121.gif` | 18.10 ms | 76.43 | 12.23× | 28.97 MiB | 27.00 MiB | 0.61 ms |
| `869910.gif` | 18.04 ms | 77.31 | 6.57× | 28.93 MiB | 27.01 MiB | 0.89 ms |
| `1q3f7z5fv7hf1.gif` | 18.57 ms | 77.58 | 8.51× | 28.81 MiB | 26.98 MiB | 0.53 ms |

The measured ten-loop phases took 3.14 s, 0.78 s and 33.38 s respectively. Including warm-up,
3,179 frames were verified. All nine additional replacement preparations matched their source's
first frame. Every cancellation/replacement returned admission to zero. Parent RSS after
warm-up and at the end stayed at approximately 11.18, 11.28 and 11.29 MiB respectively;
holding all three leases deliberately raises it to approximately 27 MiB. Allocator/kernel
accounting and sampling intervals limit the precision of these snapshots.

The 128 MiB child address-space cap admitted all three sources. Each worker reserved about
152.7 MiB: that cap plus three source RGBA frames and 1 MiB of control allowance. This is a
conservative admission charge, not observed RSS. The measured throughput exceeds each
source's average required rate, but does not establish worst-frame latency or desktop
performance under simultaneous sources, GPU uploads and transitions.

## Input identities

| File | SHA-256 |
| --- | --- |
| `775121.gif` | `1899b2b2a9811ceac9476f6ad2f5bdebe99660c67d63647df6d16509f1fae680` |
| `869910.gif` | `d1c948ede5a4a32fc421042f865aad9decb5c265e3b7cbd9c2bb4ec9657595da` |
| `1q3f7z5fv7hf1.gif` | `f5d0e6f0439768b2a4d35d9549757b19850da374a124b42ef450e8f394eedf1c` |

Still required: larger/lossy/alpha-heavy WebP, concurrent source admission and throughput,
longer soaks and replacement sweeps, persistent GPU presentation and full Wayland lifecycle
checks. These successful GIF runs do not select shipping budgets or lift the animation gate.
