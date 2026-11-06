use sigil::prelude::*;
use sigil::scene::SceneBuilder;
use sigil::widgets::{canvas, path};
use sigil_kittest::Harness;

use super::frame_of;

const SIDE: u32 = 40;
const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

fn half_covered_square() -> impl IntoNode {
    canvas(|scene: &mut SceneBuilder, rect: sigil::geometry::Rect| {
        path(SIDE, SIDE)
            .move_to(10.5, 10.5)
            .line_to(20.5, 10.5)
            .line_to(20.5, 20.5)
            .line_to(10.5, 20.5)
            .close()
            .fill(WHITE)
            .draw(scene, rect, &Theme::light());
    })
    .width(SIDE as f32)
    .height(SIDE as f32)
}

fn decode(channel: u8) -> f32 {
    let c = f32::from(channel) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn encode(linear: f32) -> f32 {
    let c = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    c * 255.0
}

#[track_caller]
fn assert_linear_edge(frame: &image::RgbaImage, what: &str) {
    let background = frame.get_pixel(30, 30)[0];
    let edge = frame.get_pixel(10, 15)[0];
    let expected = encode(0.5 + 0.5 * decode(background));
    println!("{what}: background {background}, half-covered edge {edge}, linear blend {expected:.1}");
    assert_eq!(frame.get_pixel(15, 15)[0], 255, "{what}: the square's inside is white");
    assert!(
        (f32::from(edge) - expected).abs() <= 3.0,
        "{what}: a half-covered white edge over {background} is {edge}; blended in linear light it is {expected:.1}"
    );
}

#[test]
fn a_half_covered_path_edge_blends_in_linear_light_on_pixels() {
    let mut ui = Harness::new(half_covered_square).pixels(SIDE, SIDE);
    assert_linear_edge(&frame_of(&mut ui), "pixels");
}

#[test]
fn a_half_covered_path_edge_blends_in_linear_light_on_the_gpu() {
    let mut ui = Harness::new(half_covered_square).gpu(SIDE, SIDE, 1.0);
    assert_linear_edge(&frame_of(&mut ui), "gpu");
}
