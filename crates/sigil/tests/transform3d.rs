#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small"
)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]

use std::f32::consts::PI;

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{CornerRadii, DrawRect, LayerEffect, LayerScope, SceneBuilder, Transform3D};
use sigil::style::Color;
use sigil::widgets::canvas;
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, cpu_rgba};

#[path = "transform3d/frame_time.rs"]
mod frame_time;
#[path = "transform3d/setters.rs"]
mod setters;

const VIEW: u32 = 300;
const SCALE: f32 = 2.0;
const SIDE: f32 = 100.0;
const CENTRE: f32 = 150.0;
const DISTANCE: f32 = 500.0;
const PROBE: f32 = 2.0;
const INK: Rgba8 = Rgba8 {
    r: 255,
    g: 40,
    b: 40,
    a: 255,
};

fn card() -> Rect {
    let corner = CENTRE - SIDE / 2.0;
    Rect::new(Point::new(corner, corner), SIDE, SIDE)
}

fn turned(radians: f32) -> Transform3D {
    let centre = Point::new(CENTRE, CENTRE);
    let flat = Transform3D::IDENTITY;
    flat.around(centre, flat.perspective(DISTANCE))
        .then(flat.around(centre, flat.rotate_y(radians)))
}

fn record(scene: &mut SceneBuilder, transform: Transform3D, hide_backface: bool) {
    let scope = LayerScope {
        clip: card(),
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Projected {
            transform,
            hide_backface,
        },
        opacity: 1.0,
        blend: sigil::scene::BlendMode::Normal,
    };
    scene.with_composite_layer(scope, |scene| scene.draw_rect(DrawRect::new(card(), INK)));
}

fn card_canvas(radians: f32, hide_backface: bool) -> impl Fn() -> NodeBuilder {
    move || {
        let side = VIEW as f32;
        container().child(
            canvas(move |scene, _| record(scene, turned(radians), hide_backface))
                .layout(LayoutStyle::block().width(side).height(side)),
        )
    }
}

fn css_corner(u: f32, v: f32, radians: f32) -> Point {
    let (sin, cos) = radians.sin_cos();
    let depth = -u * sin;
    let w = 1.0 - depth / DISTANCE;
    Point::new(CENTRE + u * cos / w, CENTRE + v / w)
}

fn css_quad(radians: f32) -> [Point; 4] {
    let half = SIDE / 2.0;
    [(-half, -half), (half, -half), (half, half), (-half, half)]
        .map(|(u, v)| css_corner(u, v, radians))
}

fn inside(quad: &[Point; 4], point: Point) -> bool {
    (0..4).all(|i| {
        let (a, b) = (quad[i], quad[(i + 1) % 4]);
        (b.y - a.y).mul_add(-(point.x - a.x), (b.x - a.x) * (point.y - a.y)) > 0.0
    })
}

fn edge_probes(quad: &[Point; 4]) -> Vec<(Point, bool)> {
    let centre = Point::new(CENTRE, CENTRE);
    let mut probes = Vec::new();
    for i in 0..4 {
        let (a, b) = (quad[i], quad[(i + 1) % 4]);
        let mid = Point::new(f32::midpoint(a.x, b.x), f32::midpoint(a.y, b.y));
        for point in [a, mid] {
            let (dx, dy) = (point.x - centre.x, point.y - centre.y);
            let length = dx.hypot(dy);
            let step = |sign: f32| {
                let reach = sign * PROBE * if point == mid { 1.0 } else { 2.0 };
                Point::new(
                    (dx / length).mul_add(reach, point.x),
                    (dy / length).mul_add(reach, point.y),
                )
            };
            probes.push((step(-1.0), true));
            probes.push((step(1.0), false));
        }
    }
    probes.push((Point::new(CENTRE - 36.0, CENTRE - 52.0), true));
    probes.push((Point::new(CENTRE + 40.0, CENTRE), false));
    for (point, expect) in &probes {
        assert_eq!(
            inside(quad, *point),
            *expect,
            "probe {point:?} is on the wrong side of the CSS quad"
        );
    }
    probes
}

#[track_caller]
fn frame_of(ui: &mut Harness) -> image::RgbaImage {
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no renderer: the harness returned no frame"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

fn coverage(frame: &image::RgbaImage, point: Point) -> f32 {
    let background = *frame.get_pixel(2, 2);
    let pixel = *frame.get_pixel((point.x * SCALE) as u32, (point.y * SCALE) as u32);
    let distance = |a: image::Rgba<u8>, b: [u8; 3]| -> f32 {
        a.0.iter()
            .zip(b)
            .map(|(x, y)| f32::from(x.abs_diff(y)))
            .sum()
    };
    let background_rgb = [background[0], background[1], background[2]];
    distance(pixel, background_rgb)
        / distance(image::Rgba([INK.r, INK.g, INK.b, 255]), background_rgb)
}

#[track_caller]
fn assert_corners(frame: &image::RgbaImage, surface: &str) {
    for (point, expect) in edge_probes(&css_quad(PI / 4.0)) {
        let found = coverage(frame, point);
        let ok = if expect { found >= 0.9 } else { found <= 0.1 };
        assert!(
            ok,
            "{surface}: probe ({:.1}, {:.1}) has ink {found:.2}, expected {}",
            point.x,
            point.y,
            if expect {
                "inside the projected card"
            } else {
                "outside it"
            }
        );
    }
}

#[test]
fn a_card_turned_45_degrees_under_perspective_500_lands_on_the_css_corners() {
    let mut ui = Harness::new(card_canvas(PI / 4.0, false))
        .pixels(VIEW, VIEW)
        .scale(SCALE);
    assert_corners(&frame_of(&mut ui), "cpu");
}

#[test]
fn a_card_turned_45_degrees_lands_on_the_css_corners_on_the_gpu() {
    let mut ui = Harness::new(card_canvas(PI / 4.0, false)).gpu(VIEW, VIEW, SCALE);
    assert_corners(&frame_of(&mut ui), "gpu");
}

#[test]
fn a_card_turned_to_its_back_with_backface_hidden_draws_nothing() {
    for (hide, expect) in [(true, false), (false, true)] {
        let mut ui = Harness::new(card_canvas(PI, hide))
            .pixels(VIEW, VIEW)
            .scale(SCALE);
        let found = coverage(&frame_of(&mut ui), Point::new(CENTRE, CENTRE));
        assert_eq!(
            found >= 0.9,
            expect,
            "rotateY(180deg) with hide_backface {hide}: centre ink {found:.2}"
        );
    }
}

#[test]
fn a_turned_card_agrees_on_the_cpu_and_the_gpu() {
    let mut ui = Harness::new(card_canvas(PI / 4.0, false)).gpu(VIEW, VIEW, SCALE);
    let gpu = frame_of(&mut ui);
    let viewport = (VIEW as f32, VIEW as f32);
    let scene = ui.host().emit(viewport);
    let side = (VIEW as f32 * SCALE) as u32;
    let frame = Frame {
        scene: &scene,
        viewport,
        size: PhysicalSize::new(side, side),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    let (mut worst, mut far, mut total) = (0_u8, 0_usize, 0_usize);
    for (a, b) in gpu.as_raw().chunks(4).zip(cpu.chunks(4)) {
        let delta = a
            .iter()
            .zip(b)
            .take(3)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        worst = worst.max(delta);
        far += usize::from(delta > 8);
        total += 1;
    }
    let share = far as f32 / total as f32;
    println!(
        "cpu/gpu: worst channel {worst}/255, {:.3}% of pixels differ by more than 8",
        share * 100.0
    );
    assert!(
        share <= 0.005,
        "cpu and gpu differ on {:.3}% of pixels",
        share * 100.0
    );
}
