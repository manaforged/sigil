use crate::scene_support::{frame_ms, median};

use sigil::geometry::{Point, Rect};
use sigil::kernel::{ImageSource, ObjectFit, container, image};
use sigil::layout::{FlexWrap, LayoutStyle, Overflow};
use sigil::prelude::*;
use sigil::scene::{Clip, CommandScene, DrawImage, SceneBuilder};
use sigil::style::{Paint, Radius};
use sigil_kittest::Harness;
use sigil_kittest::gpu::headless_renderer;

const COLS: u32 = 20;
const ROWS: u32 = 10;
const CELL: f32 = 50.0;
const AVATAR: f32 = 44.0;
const VIEW: (f32, f32) = (COLS as f32 * CELL, ROWS as f32 * CELL);
const SCALE: f32 = 2.0;
const WARM: usize = 5;
const ROUNDS: usize = 40;

fn source(index: u32) -> ImageSource {
    let side = (AVATAR * SCALE) as u32;
    let shade = (index * 29 % 200) as u8 + 40;
    let pixels = [shade, 255 - shade, 120, 255].repeat((side * side) as usize);
    ImageSource::from_rgba8(side, side, pixels).expect("avatar image")
}

fn slot(index: u32) -> Rect {
    let (col, row) = (index % COLS, index / COLS);
    let inset = (CELL - AVATAR) * 0.5;
    Rect::new(
        Point::new(
            (col as f32).mul_add(CELL, inset),
            (row as f32).mul_add(CELL, inset),
        ),
        AVATAR,
        AVATAR,
    )
}

fn built(shape: fn(Rect) -> Clip) -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLS * ROWS {
        let rect = slot(index);
        scene.clip(shape(rect), |scene| {
            scene.draw_image(DrawImage {
                node: None,
                source: source(index % 8),
                fit: ObjectFit::Fill,
                destination: rect,
                opacity: 10_000,
                crop: None,
                raster: None,
            });
        });
    }
    scene.finish()
}

fn avatars() -> impl IntoNode {
    let mut grid = container().layout(
        LayoutStyle::flex_row()
            .fixed_size(VIEW.0, VIEW.1)
            .flex_wrap(FlexWrap::Wrap),
    );
    for index in 0..COLS * ROWS {
        let child =
            image(source(index % 8)).layout(LayoutStyle::block().fixed_size(AVATAR, AVATAR));
        let avatar = container()
            .layout(
                LayoutStyle::block()
                    .fixed_size(AVATAR, AVATAR)
                    .margin(Space::raw((CELL - AVATAR) * 0.5, "test fixture"))
                    .overflow_x(Overflow::Hidden)
                    .overflow_y(Overflow::Hidden),
            )
            .radius(Radius::Full)
            .child(child);
        grid = grid.child(avatar);
    }
    grid
}

#[test]
#[ignore = "manual: cargo test --release -p sigil-ui --test integration clips::perf -- --ignored --nocapture"]
fn two_hundred_clipped_avatars_frame_time() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        println!("no adapter: skipping");
        return;
    };
    let mut ui = Harness::new(avatars).gpu(VIEW.0 as u32, VIEW.1 as u32, SCALE);
    let emitted = ui.host().emit(VIEW);
    let scenes: [(&str, CommandScene); 4] = [
        ("rect scissor", built(Clip::Rect)),
        ("rounded overflow", emitted),
        (
            "circle clip",
            built(|rect| Clip::rounded(rect, rect.width * 0.5)),
        ),
        ("ellipse mask", built(Clip::ellipse)),
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
    let base = median(&mut samples[0].clone());
    println!(
        "200 avatars, {}x{} at {SCALE}x, {ROUNDS} interleaved rounds",
        VIEW.0, VIEW.1
    );
    for ((name, _), list) in scenes.iter().zip(&samples) {
        let mut paired: Vec<f64> = list.iter().zip(&samples[0]).map(|(a, b)| a - b).collect();
        println!(
            "  {name:<17} frame {:7.2} ms  vs scissor {:+.2} ms (scissor median {base:.2})",
            median(&mut list.clone()),
            median(&mut paired)
        );
    }
}
