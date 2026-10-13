use super::{Budget, mapping::Mapping};
use wallpicker_core::{
    render::{Geometry, View},
    settings::{FitMode, Flip},
};

#[path = "gpu_tests.rs"]
mod gpu_tests;

fn fit(fit: FitMode) -> Geometry {
    Geometry {
        fit: Some(fit),
        ..Default::default()
    }
}

#[test]
fn cover_and_contain_match_static_layout() {
    let cover = Mapping::new((300, 100), (50, 50), Geometry::default()).unwrap();
    assert_eq!(cover.source, [100., 0., 100., 100.]);
    assert_eq!(cover.destination, [0., 0., 50., 50.]);
    let contain = Mapping::new((300, 100), (60, 61), fit(FitMode::Contain)).unwrap();
    assert_eq!(contain.source, [0., 0., 300., 100.]);
    assert_eq!(contain.destination, [0., 20., 60., 20.]);
    let tall = Mapping::new((100, 300), (61, 60), fit(FitMode::Contain)).unwrap();
    assert_eq!(tall.destination, [20., 0., 20., 60.]);
}

#[test]
fn explicit_region_overrides_fit_and_applies_flips_inside_the_cut() {
    let geometry = Geometry {
        fit: Some(FitMode::Tile),
        flip: Flip { h: true, v: true },
        view: View::Region {
            x: 200,
            y: 50,
            width: 100,
            height: 50,
        },
    };
    let mapping = Mapping::new((400, 200), (1920, 1080), geometry).unwrap();
    assert_eq!(mapping.source, [200., 50., 100., 50.]);
    assert_eq!(mapping.destination, [0., 0., 1920., 1080.]);
    assert_eq!(mapping.flip, [1., 1.]);
    assert!(!mapping.tiled);
}

#[test]
fn portable_framing_recomputes_for_output_aspect_and_clamps_small_crops() {
    let frame = Geometry {
        view: View::Frame {
            zoom: 200,
            x: 1000,
            y: 0,
        },
        ..Default::default()
    };
    let wide = Mapping::new((400, 200), (400, 200), frame).unwrap();
    let tall = Mapping::new((400, 200), (200, 400), frame).unwrap();
    assert_eq!(wide.source, [200., 0., 200., 100.]);
    assert_eq!(tall.source, [350., 0., 50., 100.]);
    let tiny = Mapping::new(
        (1, 1),
        (100, 100),
        Geometry {
            view: View::Frame {
                zoom: 400,
                x: 1000,
                y: 1000,
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(tiny.source, [0., 0., 1., 1.]);
}

#[test]
fn fill_and_tile_preserve_full_source_coordinates() {
    for mode in [FitMode::Fill, FitMode::Tile] {
        let mapping = Mapping::new((3, 2), (7, 5), fit(mode)).unwrap();
        assert_eq!(mapping.source, [0., 0., 3., 2.]);
        assert_eq!(mapping.destination, [0., 0., 7., 5.]);
        assert_eq!(mapping.tiled, mode == FitMode::Tile);
    }
}

#[test]
fn invalid_geometry_and_dimensions_are_rejected() {
    for (source, output) in [
        ((0, 1), (1, 1)),
        ((1, 1), (1, 0)),
        ((u32::MAX, 1), (1, 1)),
        ((1, 1), (1, 16385)),
    ] {
        assert!(Mapping::new(source, output, Geometry::default()).is_err());
    }
    let outside = Geometry {
        view: View::Region {
            x: 1,
            y: 0,
            width: 2,
            height: 1,
        },
        ..Default::default()
    };
    assert!(Mapping::new((2, 1), (2, 1), outside).is_err());
}

#[test]
fn shared_texture_reservation_lives_until_last_owner_drops() {
    let budget = Budget::new(32);
    let reservation = std::rc::Rc::new(budget.reserve(16).unwrap());
    let other_output = reservation.clone();
    assert_eq!(budget.used(), 16);
    assert!(budget.reserve(17).is_err());
    assert!(budget.reserve(u64::MAX).is_err());
    drop(reservation);
    assert_eq!(budget.used(), 16);
    drop(other_output);
    assert_eq!(budget.used(), 0);
}
