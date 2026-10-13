use super::{fixture::*, *};
use std::time::Duration;

fn w(x: u32, pixels: &[[u8; 4]], delay: u32, blend: bool, dispose: bool) -> WebP {
    WebP {
        x,
        pixels: pixels.to_vec(),
        delay,
        blend,
        dispose,
    }
}

#[test]
fn still_webp_and_one_frame_animation_use_static_path() {
    let tmp = tempfile::tempdir().unwrap();
    for bytes in [
        still_webp(&[RED, CLEAR]),
        webp(2, &[w(0, &[RED, CLEAR], 50, false, false)], 0),
    ] {
        let src = source(tmp.path(), &bytes);
        assert_eq!(src.info().kind, Kind::Static);
        assert_eq!(src.info().format, Format::WebP);
        assert_eq!(src.representative().unwrap().as_raw(), [RED, CLEAR].as_flattened());
        let mut stream = src.frames().unwrap();
        assert_eq!(stream.next().unwrap().unwrap().delay, None);
        assert!(stream.next().is_none());
    }
}

#[test]
fn webp_composites_offsets_blending_replace_and_background_disposal() {
    let tmp = tempfile::tempdir().unwrap();
    let bytes = webp(
        4,
        &[
            w(0, &[RED, RED], 0, false, false),
            w(2, &[GREEN, GREEN], 3, false, true),
            w(0, &[[0, 0, 255, 128]; 2], 37, true, false),
            w(0, &[CLEAR, CLEAR], 123, false, false),
        ],
        2,
    );
    let src = source(tmp.path(), &bytes);
    assert_eq!(src.info().loop_count, LoopCount::Finite(2));
    assert_eq!(src.info().kind, Kind::Animated);
    let blend = [127, 0, 128, 255];
    let expected = [
        [RED, RED, CLEAR, CLEAR],
        [RED, RED, GREEN, GREEN],
        [blend, blend, CLEAR, CLEAR],
        [CLEAR; 4],
    ];
    // Reopening must clear the prior loop's canvas, even for partial first frames.
    for _ in 0..3 {
        let mut stream = src.frames().unwrap();
        for (pixels, ms) in expected.iter().zip([0, 3, 37, 123]) {
            let frame = stream.next().unwrap().unwrap();
            assert_eq!(frame.pixels.as_raw(), pixels.as_flattened());
            assert_eq!(frame.delay, Some(Duration::from_millis(ms)));
            assert_eq!(frame.playback_delay(), Some(Duration::from_millis(ms.max(10))));
        }
        assert!(stream.next().is_none());
    }
    assert_eq!(src.representative().unwrap().as_raw(), expected[0].as_flattened());
}

#[test]
fn full_canvas_blend_only_disposes_the_previous_patch() {
    let tmp = tempfile::tempdir().unwrap();
    let bytes = webp(
        4,
        &[
            w(0, &[RED; 4], 20, false, false),
            w(2, &[GREEN; 2], 20, false, true),
            w(0, &[CLEAR; 4], 20, true, false),
        ],
        0,
    );
    let src = source(tmp.path(), &bytes);
    assert_eq!(src.info().loop_count, LoopCount::Infinite);
    let last = src.frames().unwrap().last().unwrap().unwrap();
    assert_eq!(last.pixels.as_raw(), [RED, RED, CLEAR, CLEAR].as_flattened());
}

#[test]
fn webp_rejects_truncated_containers_and_excessive_frame_count() {
    let tmp = tempfile::tempdir().unwrap();
    let bytes = webp(
        4,
        &[w(0, &[RED; 4], 10, false, false), w(2, &[BLUE; 2], 10, true, false)],
        0,
    );
    let path = tmp.path().join("bad.webp");
    for end in 0..bytes.len() {
        std::fs::write(&path, &bytes[..end]).unwrap();
        assert!(
            Source::open(&path, Limits::default()).is_err(),
            "accepted truncation at {end}"
        );
    }
    std::fs::write(&path, &bytes).unwrap();
    assert!(
        Source::open(
            &path,
            Limits {
                frame_count: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut bytes = bytes;
    bytes[24..27].copy_from_slice(&[255; 3]); // VP8X width
    std::fs::write(&path, &bytes).unwrap();
    assert!(Source::open(&path, Limits::default()).is_err());
}

#[test]
fn webp_checks_embedded_dimensions_before_decode() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bytes = webp(4, &[w(0, &[RED; 4], 10, false, false)], 0);
    let vp8l = bytes.windows(4).position(|b| b == b"VP8L").unwrap();
    bytes[vp8l + 9] = 255; // encoded VP8L width disagrees with ANMF
    let path = tmp.path().join("bad.webp");
    std::fs::write(&path, bytes).unwrap();
    assert!(Source::open(&path, Limits::default()).is_err());
}

// Generated once with ffmpeg's libwebp encoder: red 2×2, lossy. Kept inline so
// qualification tests do not depend on an installed encoder or external files.
const LOSSY_RED: &[u8] = &[
    82, 73, 70, 70, 60, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 32, 48, 0, 0, 0, 208, 1, 0, 157, 1, 42, 2, 0, 2, 0, 2, 0,
    52, 37, 160, 2, 116, 186, 1, 248, 0, 3, 176, 0, 254, 240, 232, 247, 255, 32, 185, 97, 117, 200, 215, 255, 32, 63,
    228, 7, 252, 128, 255, 248, 242, 0, 0, 0,
];

#[test]
fn lossy_webp_and_alpha_patches_decode_to_rgba() {
    let tmp = tempfile::tempdir().unwrap();
    let src = source(tmp.path(), LOSSY_RED);
    let reference = src.representative().unwrap();
    assert_eq!(reference.dimensions(), (2, 2));
    for pixel in reference.pixels() {
        assert!(pixel[0] > 245 && pixel[1] < 10 && pixel[2] < 10);
        assert_eq!(pixel[3], 255);
    }
    let mut body = b"WEBP".to_vec();
    chunk(&mut body, b"VP8X", &[0x12, 0, 0, 0, 1, 0, 0, 1, 0, 0]);
    chunk(&mut body, b"ANIM", &[0; 6]);
    let mut frame = vec![0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 10, 0, 0, 2];
    frame.extend_from_slice(&LOSSY_RED[12..]);
    chunk(&mut body, b"ANMF", &frame);
    frame.truncate(16);
    // Uncompressed, unfiltered alpha; four pixels in row-major order.
    chunk(&mut frame, b"ALPH", &[0, 0, 128, 255, 64]);
    frame.extend_from_slice(&LOSSY_RED[12..]);
    chunk(&mut body, b"ANMF", &frame);
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&body);
    let src = source(tmp.path(), &bytes);
    let mut frames = src.frames().unwrap();
    assert_eq!(frames.next().unwrap().unwrap().pixels, reference);
    let pixels = frames.next().unwrap().unwrap().pixels;
    for ((p, r), alpha) in pixels.pixels().zip(reference.pixels()).zip([0, 128, 255, 64]) {
        assert_eq!(&p.0[..3], &r.0[..3]);
        assert_eq!(p[3], alpha);
    }
    assert!(frames.next().is_none());
}

#[test]
fn malformed_later_webp_frame_does_not_invalidate_an_owned_good_frame() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bytes = webp(
        2,
        &[w(0, &[RED; 2], 20, false, false), w(0, &[GREEN; 2], 20, false, false)],
        0,
    );
    let second = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, b)| *b == b"VP8L")
        .nth(1)
        .unwrap()
        .0;
    let size = u32::from_le_bytes(bytes[second + 4..second + 8].try_into().unwrap()) as usize;
    bytes[second + 13..second + 8 + size].fill(0xff); // leave codec dimensions intact
    let src = source(tmp.path(), &bytes);
    let mut frames = src.frames().unwrap();
    let first = frames.next().unwrap().unwrap();
    assert!(frames.next().unwrap().is_err());
    assert!(frames.next().is_none());
    assert_eq!(first.pixels.as_raw(), [RED; 2].as_flattened());
}

#[test]
fn webp_compositing_budget_checks_multiple_canvases() {
    let tmp = tempfile::tempdir().unwrap();
    let bytes = webp(
        4,
        &[w(0, &[RED; 4], 20, false, false), w(0, &[GREEN; 4], 20, false, false)],
        0,
    );
    let path = tmp.path().join("limited.webp");
    std::fs::write(&path, &bytes).unwrap();
    let src = Source::open(
        &path,
        Limits {
            decoder_bytes: 16,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(src.frames().is_err());
}
