use std::io::Cursor;
use std::time::{Duration, SystemTime};

use super::{fixture::*, *};
use gif::DisposalMethod::{Any, Background, Keep, Previous};

fn g(x: u16, pixels: &[u8], disposal: gif::DisposalMethod, delay: u16) -> Gif {
    Gif {
        x,
        pixels: pixels.to_vec(),
        disposal,
        delay,
    }
}

#[test]
fn gif_composites_offsets_transparency_and_all_disposal_modes() {
    let tmp = tempfile::tempdir().unwrap();
    let data = gif(
        3,
        &[
            g(0, &[1, 1, 1], Keep, 1),
            g(1, &[2], Previous, 0),
            g(2, &[3], Background, 3),
            g(0, &[0], Keep, 7),
            g(1, &[2], Any, 2),
            g(0, &[3], Keep, 4),
        ],
        Some(gif::Repeat::Finite(2)),
    );
    let src = source(tmp.path(), &data);
    assert_eq!(src.info().format, Format::Gif);
    assert_eq!(src.info().kind, Kind::Animated);
    assert_eq!(src.info().loop_count, LoopCount::Finite(3));
    assert_eq!(src.info().frame_count, 6);
    let expected = [
        [RED, RED, RED],
        [RED, GREEN, RED],
        [RED, RED, BLUE],
        [RED, RED, CLEAR],
        [RED, GREEN, CLEAR],
        [BLUE, GREEN, CLEAR],
    ];
    let mut stream = src.frames().unwrap();
    for (pixels, ms) in expected.iter().zip([10, 0, 30, 70, 20, 40]) {
        let frame = stream.next().unwrap().unwrap();
        assert_eq!(frame.pixels.as_raw(), pixels.as_flattened());
        assert_eq!(frame.delay, Some(Duration::from_millis(ms)));
        assert_eq!(frame.playback_delay(), Some(Duration::from_millis(ms.max(10))));
    }
    assert!(stream.next().is_none());
    assert!(stream.next().is_none());
    assert_eq!(src.representative().unwrap().as_raw(), expected[0].as_flattened());
    assert_eq!(
        src.frames().unwrap().next().unwrap().unwrap().pixels,
        src.representative().unwrap()
    );
}

#[test]
fn single_gif_is_static_and_first_frame_keeps_transparency() {
    let tmp = tempfile::tempdir().unwrap();
    let src = source(tmp.path(), &gif(3, &[g(1, &[1], Keep, 0)], None));
    assert_eq!(src.info().kind, Kind::Static);
    assert_eq!(src.info().loop_count, LoopCount::Finite(1));
    let frame = src.frames().unwrap().next().unwrap().unwrap();
    assert_eq!(frame.pixels.as_raw(), [CLEAR, RED, CLEAR].as_flattened());
    assert_eq!(frame.delay, None);
    assert_eq!(frame.playback_delay(), None);
}

#[test]
fn static_formats_are_detected_by_content() {
    let tmp = tempfile::tempdir().unwrap();
    for (image_format, format) in [
        (image::ImageFormat::Png, Format::Png),
        (image::ImageFormat::Jpeg, Format::Jpeg),
    ] {
        let mut encoded = Cursor::new(Vec::new());
        image::RgbImage::from_pixel(4, 2, image::Rgb([200, 30, 10]))
            .write_to(&mut encoded, image_format)
            .unwrap();
        let src = source(tmp.path(), encoded.get_ref());
        assert_eq!(src.info().format, format);
        assert_eq!(src.info().kind, Kind::Static);
        assert_eq!(src.representative().unwrap().dimensions(), (4, 2));
        let mut frames = src.frames().unwrap();
        assert_eq!(frames.next().unwrap().unwrap().delay, None);
        assert!(frames.next().is_none());
    }
    let path = tmp.path().join("unsupported.png");
    std::fs::write(&path, b"not an image at all").unwrap();
    assert!(Source::open(&path, Limits::default()).is_err());
    assert!(Source::open(tmp.path(), Limits::default()).is_err());
}

#[test]
fn limits_reject_before_decoding_and_cover_overflow() {
    let tmp = tempfile::tempdir().unwrap();
    let data = gif(3, &[g(0, &[1, 2, 3], Keep, 1), g(0, &[2], Keep, 2)], None);
    let path = tmp.path().join("fixture");
    std::fs::write(&path, &data).unwrap();
    for limits in [
        Limits {
            encoded_bytes: data.len() as u64 - 1,
            ..Limits::default()
        },
        Limits {
            dimension: 2,
            ..Limits::default()
        },
        Limits {
            frame_bytes: 11,
            ..Limits::default()
        },
        Limits {
            decoder_bytes: 11,
            ..Limits::default()
        },
        Limits {
            frame_count: 1,
            ..Limits::default()
        },
    ] {
        assert!(Source::open(&path, limits).is_err());
    }
    let unlimited = Limits {
        dimension: u32::MAX,
        frame_bytes: u64::MAX,
        decoder_bytes: u64::MAX,
        ..Limits::default()
    };
    assert!(unlimited.dimensions(u32::MAX, u32::MAX).is_err());
    assert!(unlimited.dimensions(0, 1).is_err());
    assert!(Limits::default().dimensions(3840, 2160).is_ok());
}

#[test]
fn gif_rejects_truncation_and_out_of_canvas_patches() {
    let tmp = tempfile::tempdir().unwrap();
    let data = gif(3, &[g(0, &[1, 2, 3], Keep, 1)], None);
    let path = tmp.path().join("bad.gif");
    for end in 0..data.len() {
        std::fs::write(&path, &data[..end]).unwrap();
        assert!(
            Source::open(&path, Limits::default()).is_err(),
            "accepted truncation at {end}"
        );
    }
    let mut data = data;
    data[6..8].copy_from_slice(&1u16.to_le_bytes()); // canvas smaller than its patch
    std::fs::write(&path, data).unwrap();
    assert!(Source::open(&path, Limits::default()).is_err());
}

#[test]
fn structurally_valid_but_broken_payload_yields_one_error() {
    let tmp = tempfile::tempdir().unwrap();
    let mut data = gif(3, &[g(0, &[1, 2, 3], Keep, 1)], None);
    // Replace the LZW minimum code size (immediately after the descriptor)
    // with another legal size, leaving the compressed codes inconsistent.
    let descriptor = data.iter().position(|b| *b == 0x2c).unwrap();
    data[descriptor + 10] = 8;
    let src = source(tmp.path(), &data);
    let mut frames = src.frames().unwrap();
    assert!(frames.next().unwrap().is_err());
    assert!(frames.next().is_none());
    assert!(frames.next().is_none());
}

#[test]
fn cache_is_bounded_and_invalidates_same_size_same_mtime_replacement() {
    let tmp = tempfile::tempdir().unwrap();
    let data = gif(3, &[g(0, &[1, 1, 1], Keep, 1)], None);
    let path = tmp.path().join("a.gif");
    std::fs::write(&path, &data).unwrap();
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(123);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(time)
        .unwrap();
    let mut cache = MetadataCache::new(Limits::default(), 1);
    let old = cache.get(&path).unwrap();
    assert_eq!(cache.get(&path).unwrap().identity(), old.identity());
    let replacement = tmp.path().join("replacement");
    let mut data = data;
    data[6..8].copy_from_slice(&4u16.to_le_bytes());
    std::fs::write(&replacement, &data).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_modified(time)
        .unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    let new = cache.get(&path).unwrap();
    assert_eq!(old.identity().len, new.identity().len);
    assert_eq!(old.identity().modified, new.identity().modified);
    assert_ne!(old.identity(), new.identity());
    assert_eq!(new.info().width, 4);
    assert!(old.frames().is_err());
    let second = tmp.path().join("b.gif");
    std::fs::write(&second, &data).unwrap();
    cache.get(&second).unwrap();
    assert_eq!(cache.len(), 1);
    cache.clear();
    assert!(cache.is_empty());
    let mut disabled = MetadataCache::new(Limits::default(), 0);
    disabled.get(&path).unwrap();
    assert!(disabled.is_empty());
}

#[test]
fn changing_a_source_mid_stream_stops_decoding() {
    let tmp = tempfile::tempdir().unwrap();
    let data = gif(
        3,
        &[g(0, &[1], Keep, 1), g(0, &[2], Keep, 1)],
        Some(gif::Repeat::Infinite),
    );
    let src = source(tmp.path(), &data);
    assert_eq!(src.info().loop_count, LoopCount::Infinite);
    let mut stream = src.frames().unwrap();
    assert!(stream.next().unwrap().is_ok());
    std::fs::write(src.path(), b"changed").unwrap();
    assert!(stream.next().unwrap().is_err());
    assert!(stream.next().is_none());
}

#[test]
fn cache_uses_subsecond_modification_times() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("a.gif");
    std::fs::write(&path, gif(1, &[g(0, &[1], Keep, 1)], None)).unwrap();
    let file = std::fs::File::options().write(true).open(&path).unwrap();
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(123);
    file.set_modified(time).unwrap();
    let mut cache = MetadataCache::new(Limits::default(), 1);
    let first = cache.get(&path).unwrap();
    file.set_modified(time + Duration::from_nanos(1)).unwrap();
    let second = cache.get(&path).unwrap();
    assert_ne!(first.identity().modified, second.identity().modified);
    assert!(first.frames().is_err());
    assert!(second.representative().is_ok());
}
