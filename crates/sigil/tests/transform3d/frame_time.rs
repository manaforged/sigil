use std::f32::consts::PI;
use std::time::{Duration, Instant};

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::renderer::CpuRenderer;
use sigil::scene::{
    CommandScene, CornerRadii, DrawRect, LayerEffect, LayerScope, SceneBuilder, Transform3D,
};
use sigil::style::Color;
use sigil_kittest::gpu::{headless_renderer, render_scene_to_image};
use sigil_kittest::parity::{Frame, cpu_rgba};

use super::{INK, SCALE, SIDE};

const COLUMNS: usize = 10;
const ROWS: usize = 5;
const PITCH: f32 = 110.0;
const ROUNDS: usize = 40;
const VIEW: (f32, f32) = (PITCH * COLUMNS as f32, PITCH * ROWS as f32);

#[derive(Clone, Copy, Debug)]
enum Cards {
    Flat,
    Turned2D,
    Turned3D,
}

fn effect(cards: Cards, rect: Rect) -> LayerEffect {
    let centre = Point::new(rect.origin.x + SIDE / 2.0, rect.origin.y + SIDE / 2.0);
    match cards {
        Cards::Flat => LayerEffect::Identity,
        Cards::Turned2D => {
            let (sin, cos) = (PI / 4.0).sin_cos();
            LayerEffect::Transform([
                [
                    cos,
                    -sin,
                    sin.mul_add(centre.y, cos.mul_add(-centre.x, centre.x)),
                ],
                [
                    sin,
                    cos,
                    cos.mul_add(-centre.y, sin.mul_add(-centre.x, centre.y)),
                ],
            ])
        }
        Cards::Turned3D => {
            let flat = Transform3D::IDENTITY;
            LayerEffect::Projected {
                transform: flat
                    .around(centre, flat.perspective(500.0))
                    .then(flat.around(centre, flat.rotate_y(PI / 4.0))),
                hide_backface: false,
            }
        }
    }
}

fn scene(cards: Cards) -> CommandScene {
    let mut scene = SceneBuilder::new();
    for index in 0..COLUMNS * ROWS {
        let at = Point::new(
            ((index % COLUMNS) as f32).mul_add(PITCH, 5.0),
            ((index / COLUMNS) as f32).mul_add(PITCH, 5.0),
        );
        let rect = Rect::new(at, SIDE, SIDE);
        let scope = LayerScope {
            clip: rect,
            clip_radii: CornerRadii::ZERO,
            effect: effect(cards, rect),
            opacity: 1.0,
            blend: sigil::scene::BlendMode::Normal,
        };
        scene.with_composite_layer(scope, |scene| scene.draw_rect(DrawRect::new(rect, INK)));
    }
    scene.finish()
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples.get(samples.len() / 2).copied().unwrap_or_default()
}

fn device_size() -> [u32; 2] {
    [VIEW.0, VIEW.1].map(|side| (side * SCALE) as u32)
}

fn cpu_frame(renderer: &mut CpuRenderer, scene: &CommandScene) -> Duration {
    let [width, height] = device_size();
    let frame = Frame {
        scene,
        viewport: VIEW,
        size: PhysicalSize::new(width, height),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let start = Instant::now();
    let _ = cpu_rgba(renderer, &frame).expect("cpu render");
    start.elapsed()
}

#[test]
#[ignore = "manual: frame time of 50 cards; run with --release -- --ignored --nocapture"]
fn fifty_turned_cards_frame_time() {
    let kinds = [Cards::Flat, Cards::Turned2D, Cards::Turned3D];
    let scenes = kinds.map(scene);
    let mut gpu = headless_renderer().expect("gpu").expect("a gpu adapter");
    let mut cpu = CpuRenderer::new();
    let mut gpu_times = kinds.map(|_| Vec::new());
    let mut cpu_times = kinds.map(|_| Vec::new());
    for round in 0..ROUNDS + 3 {
        for (index, scene) in scenes.iter().enumerate() {
            let start = Instant::now();
            let _ =
                render_scene_to_image(&mut gpu, scene, VIEW, device_size()).expect("gpu render");
            let gpu_time = start.elapsed();
            let cpu_time = cpu_frame(&mut cpu, scene);
            if round >= 3 {
                gpu_times[index].push(gpu_time);
                cpu_times[index].push(cpu_time);
            }
        }
    }
    for (index, kind) in kinds.iter().enumerate() {
        println!(
            "{kind:?}: gpu median {:?}, cpu median {:?} over {ROUNDS} paired rounds",
            median(std::mem::take(&mut gpu_times[index])),
            median(std::mem::take(&mut cpu_times[index])),
        );
    }
}
