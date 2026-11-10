use std::time::Instant;

use sigil::geometry::{Point, Rect};
use sigil::renderer::WgpuRenderer;
use sigil::scene::{
    CommandScene, CornerRadii, DrawRect, LayerEffect, LayerScope, Mask, SceneBuilder,
};
use sigil::style::Color;
use sigil_kittest::gpu::{Capture, headless_renderer, read_pixels, render_capture, shared_device};

use super::{RED, alpha_ramp, image_mask, luminance_ramp, masked_red};

const COLS: u32 = 10;
const ROWS: u32 = 10;
const CELL: f32 = 80.0;
const CARD: f32 = 70.0;
const VIEW: (f32, f32) = (COLS as f32 * CELL, ROWS as f32 * CELL);
const SCALE: f32 = 2.0;
const WARM: usize = 5;
const ROUNDS: usize = 40;

fn slot(index: u32) -> Rect {
    let (col, row) = (index % COLS, index / COLS);
    let inset = (CELL - CARD) * 0.5;
    Rect::new(
        Point::new(
            (col as f32).mul_add(CELL, inset),
            (row as f32).mul_add(CELL, inset),
        ),
        CARD,
        CARD,
    )
}

fn built(mask: Option<fn(Rect) -> Mask>) -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = slot(index);
        match mask {
            Some(mask) => masked_red(&mut scene, rect, mask(rect)),
            None => scene.draw_rect(DrawRect::new(rect, RED)),
        }
    }
    scene.finish()
}

fn layered() -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = slot(index);
        let scope = LayerScope {
            clip: rect,
            clip_radii: CornerRadii::ZERO,
            effect: LayerEffect::Identity,
            opacity: 0.99,
            blend: sigil::scene::BlendMode::Normal,
        };
        scene.with_composite_layer(scope, |scene| scene.draw_rect(DrawRect::new(rect, RED)));
    }
    scene.finish()
}

fn frame_ms(renderer: &mut WgpuRenderer, scene: &CommandScene) -> f64 {
    let (device, queue) = shared_device().expect("sigil-kittest").expect("adapter");
    let capture = Capture {
        viewport: VIEW,
        size_px: [(VIEW.0 * SCALE) as u32, (VIEW.1 * SCALE) as u32],
        scale: SCALE,
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let start = Instant::now();
    let texture = render_capture(renderer, (device, queue), scene, &capture).expect("render");
    let [w, h] = capture.size_px;
    std::hint::black_box(read_pixels(device, queue, &texture, w, h).expect("readback"));
    start.elapsed().as_secs_f64() * 1e3
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(f64::NAN)
}

#[test]
#[ignore = "manual: cargo test --release -p sigil-ui --test integration masks::perf -- --ignored --nocapture"]
fn a_hundred_masked_cards_frame_time() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        println!("no adapter: skipping");
        return;
    };
    let scenes: [(&str, CommandScene); 5] = [
        ("unmasked", built(None)),
        ("opacity layer", layered()),
        ("alpha gradient", built(Some(alpha_ramp))),
        ("luminance", built(Some(luminance_ramp))),
        ("image", built(Some(image_mask))),
    ];
    let mut samples: Vec<Vec<f64>> = scenes.iter().map(|_| Vec::new()).collect();
    for round in 0..WARM + ROUNDS {
        for offset in 0..scenes.len() {
            let pick = (round + offset) % scenes.len();
            let ms = frame_ms(&mut renderer, &scenes[pick].1);
            if round >= WARM {
                samples[pick].push(ms);
            }
        }
    }
    println!(
        "100 cards, {}x{} at {SCALE}x, {ROUNDS} interleaved rounds",
        VIEW.0, VIEW.1
    );
    for ((name, _), list) in scenes.iter().zip(&samples) {
        let mut paired: Vec<f64> = list.iter().zip(&samples[0]).map(|(a, b)| a - b).collect();
        println!(
            "  {name:<15} frame {:7.2} ms  vs unmasked {:+.2} ms",
            median(&mut list.clone()),
            median(&mut paired)
        );
    }
}
