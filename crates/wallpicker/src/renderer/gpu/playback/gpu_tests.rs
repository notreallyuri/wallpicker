//! Real shader/texture checks via a headless EGL pbuffer. Readback exists only
//! in this test harness; it never touches a compositor or wallpaper surface.
use super::*;
use crate::renderer::gpu::{
    Device,
    playback::{Painter, Texture},
    program,
};
use crate::renderer::transition::{Effect, Transition};
use glow::HasContext;
use khronos_egl as egl;
use std::rc::Rc;
use wallpicker_core::media::RgbaImage;

struct Buffer {
    painter: Rc<Painter>,
    surface: egl::Surface,
    size: (u32, u32),
}
impl Buffer {
    fn new(painter: Rc<Painter>, size: (u32, u32)) -> Self {
        painter.validate_output(size).unwrap();
        let c = &painter.device.context;
        let surface = c
            .api
            .create_pbuffer_surface(
                c.display,
                c.config,
                &[egl::WIDTH, size.0 as i32, egl::HEIGHT, size.1 as i32, egl::NONE],
            )
            .unwrap();
        Self { painter, surface, size }
    }
    fn draw(&self, texture: &Texture, geometry: Geometry) -> RgbaImage {
        self.draw_effect(texture, geometry, Effect::Fade, 1.0)
    }
    fn draw_effect(&self, texture: &Texture, geometry: Geometry, effect: Effect, progress: f64) -> RgbaImage {
        self.painter.device.context.current(Some(self.surface)).unwrap();
        self.painter
            .draw_effect(texture, self.size, geometry, effect, progress)
            .unwrap();
        let (w, h) = self.size;
        let mut pixels = vec![0; (w * h * 4) as usize];
        // SAFETY: current pbuffer has these dimensions; output is RGBA-sized.
        unsafe {
            self.painter.device.gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        super::super::check(&self.painter.device.gl, "test readback").unwrap();
        // Convert GLES's bottom-first readback to the source's top-first rows.
        let mut top_first = vec![0; pixels.len()];
        let stride = w as usize * 4;
        for y in 0..h as usize {
            top_first[y * stride..(y + 1) * stride]
                .copy_from_slice(&pixels[(h as usize - y - 1) * stride..(h as usize - y) * stride]);
        }
        RgbaImage::from_raw(w, h, top_first).unwrap()
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        let c = &self.painter.device.context;
        c.current(None).unwrap();
        c.api.destroy_surface(c.display, self.surface).unwrap();
    }
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

#[track_caller]
fn near(actual: [u8; 4], expected: [u8; 4]) {
    assert!(
        actual.into_iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 2),
        "{actual:?} != {expected:?}"
    );
}

#[test]
#[ignore = "requires EGL_MESA_platform_surfaceless; accepts software GLES only in this test"]
fn headless_gpu_pixels_reuse_and_cleanup() {
    let device = Device::headless().unwrap();
    let painter = Painter::new(device.clone()).unwrap();
    let budget = Budget::new(1024);
    let image = Texture::new(device.clone(), &budget, (3, 2), 7).unwrap();
    let original_texture = image.id;
    let source = [RED, GREEN, BLUE, BLUE, RED, GREEN].as_flattened().to_vec();
    assert!(
        painter.draw(&image, (3, 2), Geometry::default()).is_err(),
        "uninitialized texture"
    );
    assert!(image.upload(7, 0, (3, 2), &source).unwrap());
    let target = Buffer::new(painter.clone(), (3, 2));
    let pixels = target.draw(&image, fit(FitMode::Fill));
    assert_eq!(pixels.as_raw(), &source);
    for (h, v) in [(true, false), (false, true), (true, true)] {
        let pixels = target.draw(
            &image,
            Geometry {
                flip: Flip { h, v },
                ..fit(FitMode::Fill)
            },
        );
        for y in 0..2 {
            for x in 0..3 {
                let (sx, sy) = (if h { 2 - x } else { x }, if v { 1 - y } else { y });
                near(
                    pixels.get_pixel(x, y).0,
                    source[((sy * 3 + sx) * 4) as usize..((sy * 3 + sx + 1) * 4) as usize]
                        .try_into()
                        .unwrap(),
                );
            }
        }
    }
    let tile = Buffer::new(painter.clone(), (7, 5));
    for flip in [Flip::default(), Flip { h: true, v: true }] {
        let pixels = tile.draw(
            &image,
            Geometry {
                flip,
                ..fit(FitMode::Tile)
            },
        );
        for y in 0..5 {
            for x in 0..7 {
                let (sx, sy) = (
                    if flip.h { 2 - x % 3 } else { x % 3 },
                    if flip.v { 1 - y % 2 } else { y % 2 },
                );
                near(
                    pixels.get_pixel(x, y).0,
                    source[((sy * 3 + sx) * 4) as usize..((sy * 3 + sx + 1) * 4) as usize]
                        .try_into()
                        .unwrap(),
                );
            }
        }
    }
    let tall = Buffer::new(painter.clone(), (3, 6));
    let contained = tall.draw(&image, fit(FitMode::Contain));
    for y in [0, 1, 4, 5] {
        for x in 0..3 {
            near(contained.get_pixel(x, y).0, BLACK);
        }
    }
    near(contained.get_pixel(0, 2).0, RED);
    near(contained.get_pixel(0, 3).0, BLUE);
    let crop = Geometry {
        view: View::Region {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        },
        ..fit(FitMode::Tile)
    };
    assert!(
        tile.draw(&image, crop).pixels().all(|p| p.0 == GREEN),
        "region edges must not bleed adjacent colors"
    );
    assert_eq!(image.uploads(), 1, "draw/geometry/output-size changes must not upload");
    assert_eq!(image.id, original_texture);
    assert!(!image.upload(7, 0, (3, 2), &source).unwrap());
    assert!(image.upload(6, 1, (3, 2), &source).is_err());
    assert!(image.upload(7, 1, (3, 2), &[0; 3]).is_err());
    assert!(image.upload(7, 1, (2, 3), &source).is_err());
    let new_frame = RED.repeat(6);
    assert!(image.upload(7, 2, (3, 2), &new_frame).unwrap());
    assert!(image.upload(7, 1, (3, 2), &source).is_err());
    assert!(target.draw(&image, Geometry::default()).pixels().all(|p| p.0 == RED));
    assert!(tile.draw(&image, fit(FitMode::Tile)).pixels().all(|p| p.0 == RED));
    assert_eq!(image.uploads(), 2);

    alpha_and_framing(&device, &painter, &budget);
    transition_pixels(&device, &painter, &budget);
    transition_shaders_still_compile(&device);
    let used = budget.used();
    let other_output = image.clone();
    drop(image);
    assert_eq!(budget.used(), used);
    drop(other_output);
    assert_eq!(budget.used(), 0);
    // SAFETY: Texture::drop leaves the scratch context current.
    assert!(!unsafe { device.gl.is_texture(original_texture) });
    assert!(Texture::new(device.clone(), &Budget::new(1), (3, 2), 7).is_err());
    assert_eq!(budget.used(), 0);
}

fn transition_pixels(device: &Rc<Device>, painter: &Rc<Painter>, budget: &Budget) {
    let image = Texture::new(device.clone(), budget, (2, 1), 99).unwrap();
    image.upload(99, 0, (2, 1), [RED, BLUE].as_flattened()).unwrap();
    let target = Buffer::new(painter.clone(), (8, 8));
    let geometries = [
        fit(FitMode::Fill),
        fit(FitMode::Contain),
        fit(FitMode::Tile),
        Geometry {
            view: View::Region {
                x: 1,
                y: 0,
                width: 1,
                height: 1,
            },
            flip: Flip { h: true, v: true },
            ..Default::default()
        },
        Geometry {
            view: View::Frame {
                zoom: 200,
                x: 750,
                y: 250,
            },
            ..Default::default()
        },
    ];
    for kind in [
        "fade", "wipe", "wave", "left", "right", "top", "bottom", "grow", "outer", "any",
    ] {
        let effect = Transition::from_setting(kind, 100).unwrap().effect;
        for geometry in geometries {
            let steady = target.draw(&image, geometry);
            assert!(
                target
                    .draw_effect(&image, geometry, effect, 0.0)
                    .pixels()
                    .all(|p| p.0 == [0; 4]),
                "{kind} start"
            );
            assert_eq!(
                target.draw_effect(&image, geometry, effect, 1.0),
                steady,
                "{kind} endpoint"
            );
            let mid = target.draw_effect(&image, geometry, effect, 0.5);
            assert!(
                mid.pixels().all(|p| p.0[..3].iter().all(|c| *c <= p.0[3])),
                "{kind} premultiplication"
            );
            match kind {
                "fade" => {
                    for (pixel, original) in mid.pixels().zip(steady.pixels()) {
                        near(pixel.0, original.0.map(|v| (f32::from(v) * 0.5).round() as u8));
                    }
                }
                "left" | "right" | "top" | "bottom" => {
                    let (dx, dy) = effect.parameters();
                    for y in 0..8 {
                        for x in 0..8 {
                            let sx = x as i32 + (dx * 4.0) as i32;
                            let sy = y as i32 + (dy * 4.0) as i32;
                            let expected = if (0..8).contains(&sx) && (0..8).contains(&sy) {
                                steady.get_pixel(sx as u32, sy as u32).0
                            } else {
                                [0; 4]
                            };
                            near(mid.get_pixel(x, y).0, expected);
                        }
                    }
                }
                "wipe" | "wave" => {
                    near(mid.get_pixel(0, 4).0, steady.get_pixel(0, 4).0);
                    near(mid.get_pixel(7, 4).0, [0; 4]);
                }
                "grow" => {
                    near(mid.get_pixel(4, 4).0, steady.get_pixel(4, 4).0);
                    near(mid.get_pixel(0, 0).0, [0; 4]);
                }
                "outer" => {
                    near(mid.get_pixel(4, 4).0, [0; 4]);
                    near(mid.get_pixel(0, 0).0, steady.get_pixel(0, 0).0);
                }
                _ => {}
            }
        }
    }
    assert_eq!(image.uploads(), 1, "effects must not upload frozen frames again");
}

fn alpha_and_framing(device: &Rc<Device>, painter: &Rc<Painter>, budget: &Budget) {
    let image = Texture::new(device.clone(), budget, (2, 1), 1).unwrap();
    image
        .upload(1, 0, (2, 1), [[255, 0, 0, 0], GREEN].as_flattened())
        .unwrap();
    let target = Buffer::new(painter.clone(), (4, 1));
    let pixels = target.draw(&image, fit(FitMode::Fill));
    for (x, green) in [0, 64, 191, 255].into_iter().enumerate() {
        near(pixels.get_pixel(x as u32, 0).0, [0, green, 0, 255]);
    }
    image
        .upload(1, 1, (2, 1), [[255, 0, 0, 128], [0, 0, 255, 0]].as_flattened())
        .unwrap();
    near(
        target.draw(&image, fit(FitMode::Fill)).get_pixel(0, 0).0,
        [128, 0, 0, 255],
    );

    let image = Texture::new(device.clone(), budget, (6, 2), 2).unwrap();
    let row = [RED, RED, GREEN, GREEN, BLUE, BLUE];
    image.upload(2, 0, (6, 2), row.repeat(2).as_flattened()).unwrap();
    let square = Buffer::new(painter.clone(), (2, 2));
    assert!(square.draw(&image, fit(FitMode::Cover)).pixels().all(|p| p.0 == GREEN));
    let frame = Geometry {
        view: View::Frame {
            zoom: 100,
            x: 1000,
            y: 500,
        },
        ..Default::default()
    };
    assert!(square.draw(&image, frame).pixels().all(|p| p.0 == BLUE));
    let pixel = Texture::new(device.clone(), budget, (1, 1), 3).unwrap();
    pixel.upload(3, 0, (1, 1), &RED).unwrap();
    assert!(target.draw(&pixel, Geometry::default()).pixels().all(|p| p.0 == RED));
}

fn transition_shaders_still_compile(device: &Rc<Device>) {
    use crate::renderer::transition::Transition;
    device.context.current(None).unwrap();
    for kind in ["wipe", "wave", "left", "right", "top", "bottom", "grow", "outer"] {
        let effect = Transition::from_setting(kind, 100).unwrap().effect;
        let program = program::compile(&device.gl, effect.shader().unwrap()).unwrap();
        // SAFETY: program belongs to this current context.
        unsafe {
            device.gl.delete_program(program);
        }
    }
}
