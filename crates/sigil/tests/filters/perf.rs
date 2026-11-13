use crate::scene_support::{frame_ms, median};

use sigil::geometry::{Point, Rect};
use sigil::scene::{CommandScene, DrawRect, Filter, LayerScope, SceneBuilder, Shadow};
use sigil_kittest::gpu::headless_renderer;

use super::{INPUT, filtered};

const COLS: u32 = 10;
const ROWS: u32 = 10;
const CELL: f32 = 80.0;
const CARD: f32 = 60.0;
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

fn card(scene: &mut SceneBuilder, rect: Rect) {
    let inner = Rect::new(
        Point::new(rect.origin.x + 8.0, rect.origin.y + 8.0),
        rect.width - 16.0,
        rect.height - 16.0,
    );
    scene.draw_rect(DrawRect::new(inner, INPUT));
}

fn built(filters: Option<&[Filter]>) -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = slot(index);
        match filters {
            Some(filters) => filtered(&mut scene, rect, filters, card),
            None => card(&mut scene, rect),
        }
    }
    scene.finish()
}

fn layered() -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = slot(index);
        let scope = LayerScope::new(rect).opacity(0.99);
        scene.composite(scope, |scene| card(scene, rect));
    }
    scene.finish()
}

#[test]
#[ignore = "manual: cargo test --release -p sigil-ui --test integration filters::perf -- --ignored --nocapture"]
fn a_hundred_filtered_cards_frame_time() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        println!("no adapter: skipping");
        return;
    };
    let shadow = Shadow::new([0.0, 4.0], 3.0, 0.0, [0.0, 0.0, 0.0, 0.3]);
    let scenes: [(&str, CommandScene); 6] = [
        ("plain", built(None)),
        ("opacity layer", layered()),
        (
            "color matrix",
            built(Some(&[Filter::Grayscale(1.0), Filter::Sepia(0.5)])),
        ),
        ("blur 4", built(Some(&[Filter::Blur(4.0)]))),
        ("drop-shadow", built(Some(&[Filter::DropShadow(shadow)]))),
        (
            "chain",
            built(Some(&[
                Filter::Saturate(1.5),
                Filter::Blur(2.0),
                Filter::DropShadow(shadow),
            ])),
        ),
    ];
    let mut samples: Vec<Vec<f64>> = scenes.iter().map(|_| Vec::new()).collect();
    for round in 0..WARM + ROUNDS {
        for offset in 0..scenes.len() {
            let pick = (round + offset) % scenes.len();
            let ms = frame_ms(&mut renderer, &scenes[pick].1, VIEW, SCALE);
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
            "  {name:<14} frame {:7.2} ms  vs plain {:+.2} ms",
            median(&mut list.clone()),
            median(&mut paired)
        );
    }
}
