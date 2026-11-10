use std::time::Instant;

use sigil::geometry::{Point, Rect};
use sigil::renderer::WgpuRenderer;
use sigil::scene::{BlendMode, CommandScene, DrawRect, SceneBuilder};
use sigil_kittest::gpu::{Capture, headless_renderer, read_pixels, render_capture, shared_device};

use super::{BACKDROP, SOURCE, blended, rgba};

const COLS: u32 = 20;
const ROWS: u32 = 10;
const SLOT: f32 = 40.0;
const CARD: f32 = 32.0;
const VIEW: (f32, f32) = (COLS as f32 * SLOT, ROWS as f32 * SLOT);
const SCALE: f32 = 2.0;
const WARM: usize = 5;
const ROUNDS: usize = 40;

fn slot(index: u32) -> Rect {
    let (col, row) = (index % COLS, index / COLS);
    Rect::new(Point::new(col as f32 * SLOT, row as f32 * SLOT), SLOT, SLOT)
}

fn card(index: u32) -> Rect {
    let outer = slot(index);
    let inset = (SLOT - CARD) * 0.5;
    Rect::new(
        Point::new(outer.origin.x + inset, outer.origin.y + inset),
        CARD,
        CARD,
    )
}

fn built(mode: Option<BlendMode>) -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        scene.draw_rect(DrawRect::new(slot(index), rgba(BACKDROP, 255)));
        let source = rgba(SOURCE, 255);
        match mode {
            Some(mode) => blended(&mut scene, card(index), mode, source),
            None => scene.draw_rect(DrawRect::new(card(index), source)),
        }
    }
    scene.finish()
}

fn frame_ms(renderer: &mut WgpuRenderer, scene: &CommandScene) -> f64 {
    let (device, queue) = shared_device().expect("sigil-kittest").expect("adapter");
    let capture =
        Capture::new(VIEW, [(VIEW.0 * SCALE) as u32, (VIEW.1 * SCALE) as u32]).scale(SCALE);
    let start = Instant::now();
    let texture = render_capture(renderer, (device, queue), scene, &capture).expect("render");
    let [w, h] = capture.size_px();
    std::hint::black_box(read_pixels(device, queue, &texture, w, h).expect("readback"));
    start.elapsed().as_secs_f64() * 1e3
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(f64::NAN)
}

#[test]
#[ignore = "manual: cargo test --release -p sigil-ui --test integration blend::perf -- --ignored --nocapture"]
fn two_hundred_blended_layers_frame_time() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        println!("no adapter: skipping");
        return;
    };
    let scenes: [(&str, CommandScene); 5] = [
        ("no layers", built(None)),
        ("normal", built(Some(BlendMode::Normal))),
        ("multiply", built(Some(BlendMode::Multiply))),
        ("soft-light", built(Some(BlendMode::SoftLight))),
        ("hue", built(Some(BlendMode::Hue))),
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
        "200 layers, {}x{} at {SCALE}x, {ROUNDS} interleaved rounds",
        VIEW.0, VIEW.1
    );
    for ((name, _), list) in scenes.iter().zip(&samples) {
        let mut paired: Vec<f64> = list.iter().zip(&samples[1]).map(|(a, b)| a - b).collect();
        println!(
            "  {name:<10} frame {:7.2} ms  vs normal {:+.2} ms",
            median(&mut list.clone()),
            median(&mut paired)
        );
    }
}
