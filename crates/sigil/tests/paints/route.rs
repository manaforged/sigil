use std::time::Instant;

use sigil::geometry::{Point, Rect};
use sigil::prelude::*;
use sigil::renderer::WgpuRenderer;
use sigil::scene::{
    CommandScene, CornerRadii, DrawGradient, Gradient, GradientKind, GradientStop, SceneBuilder,
    shape_glyph_run,
};
use sigil::style::{Color, TextStyle};
use sigil_kittest::gpu::{Capture, headless_renderer, read_pixels, render_capture, shared_device};

const VIEW: (f32, f32) = (1250.0, 800.0);
const COLS: u32 = 25;
const ROWS: u32 = 20;
const RADIUS: f32 = 10.0;
const ROUNDS: usize = 40;
const WARM: usize = 5;
const FROM: Rgba8 = super::rgb(250, 120, 40);
const TO: Rgba8 = super::rgb(40, 60, 200);

fn card(index: u32) -> Rect {
    let (w, h) = (VIEW.0 / COLS as f32, VIEW.1 / ROWS as f32);
    let (col, row) = (index % COLS, index / COLS);
    Rect::new(
        Point::new((col as f32).mul_add(w, 2.0), (row as f32).mul_add(h, 2.0)),
        w - 4.0,
        h - 4.0,
    )
}

fn label(scene: &mut SceneBuilder, rect: Rect, index: u32) {
    let style = TextStyle::default();
    scene.draw_glyph_run(shape_glyph_run(
        rect,
        format!("card {index}"),
        super::WHITE,
        style,
        10_000,
    ));
}

fn quads() -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = card(index);
        let center = Point::new(
            rect.origin.x + rect.width / 2.0,
            rect.origin.y + rect.height / 2.0,
        );
        let radius = [rect.width * 0.7, rect.height * 0.7];
        let stops = vec![GradientStop::new(0.0, FROM), GradientStop::new(1.0, TO)];
        let gradient = Gradient::new(GradientKind::Radial { center, radius }, stops);
        scene.draw_gradient(
            DrawGradient::new(rect, gradient).border_radius(CornerRadii::uniform(RADIUS)),
        );
        label(&mut scene, rect, index);
    }
    scene.finish()
}

fn paths() -> CommandScene {
    let theme = Theme::light();
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = card(index);
        let (w, h) = (rect.width, rect.height);
        let k = RADIUS * 0.447_715;
        let node = path(w as u32, h as u32)
            .move_to(RADIUS, 0.0)
            .line_to(w - RADIUS, 0.0)
            .cubic_to(w - k, 0.0, w, k, w, RADIUS)
            .line_to(w, h - RADIUS)
            .cubic_to(w, h - k, w - k, h, w - RADIUS, h)
            .line_to(RADIUS, h)
            .cubic_to(k, h, 0.0, h - k, 0.0, h - RADIUS)
            .line_to(0.0, RADIUS)
            .cubic_to(0.0, k, k, 0.0, RADIUS, 0.0)
            .close()
            .fill_radial((w / 2.0, h / 2.0), w * 0.7, vec![(0.0, FROM), (1.0, TO)]);
        node.draw(&mut scene, rect, &theme);
        label(&mut scene, rect, index);
    }
    scene.finish()
}

fn frame(renderer: &mut WgpuRenderer, scene: &CommandScene) -> f64 {
    let (device, queue) = shared_device().expect("sigil-kittest").expect("adapter");
    let capture = Capture {
        viewport: VIEW,
        size_px: [(VIEW.0 * 2.0) as u32, (VIEW.1 * 2.0) as u32],
        scale: 2.0,
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
    };
    let start = Instant::now();
    let texture = render_capture(renderer, (device, queue), scene, &capture).expect("render");
    let [w, h] = capture.size_px;
    std::hint::black_box(read_pixels(device, queue, &texture, w, h).expect("readback"));
    start.elapsed().as_secs_f64() * 1e3
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

#[test]
#[ignore = "manual: frame-time comparison of the quad and path routes, run with --release -- --ignored"]
fn a_500_card_radial_scene_costs_less_through_the_quad_route() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        println!("no adapter");
        return;
    };
    let scenes = [quads(), paths()];
    let mut times = [Vec::new(), Vec::new()];
    for round in 0..WARM + ROUNDS {
        for offset in 0..2 {
            let pick = (round + offset) % 2;
            let ms = frame(&mut renderer, &scenes[pick]);
            if round >= WARM {
                times[pick].push(ms);
            }
        }
    }
    let mut diffs: Vec<f64> = times[1].iter().zip(&times[0]).map(|(p, q)| p - q).collect();
    let [quad, route] = [median(&mut times[0].clone()), median(&mut times[1].clone())];
    let paired = median(&mut diffs);
    println!(
        "500 radial cards with labels, 2x: quad {quad:.2} ms, path {route:.2} ms, path - quad paired median {paired:+.2} ms over {ROUNDS} rounds"
    );
}
