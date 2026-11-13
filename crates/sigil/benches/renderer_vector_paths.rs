#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, reason = "the bench prints its measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell counts and pixel sides are small"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "a bench fails loudly on a missing sample"
)]

use std::time::Instant;

use sigil::geometry::{Point, Rect};
use sigil::kernel::{ImageSource, ObjectFit};
use sigil::renderer::{CpuRenderer, WgpuRenderer};
use sigil::scene::{CommandScene, DrawImage, SceneBuilder, SceneCommand};
use sigil::style::{Color, Rgba, Theme};
use sigil::widgets::path;
use sigil_kittest::gpu::{Capture, headless_renderer, read_pixels, render_capture, shared_device};
use sigil_kittest::parity::{Frame, cpu_rgba};

const VIEW: (f32, f32) = (1280.0, 800.0);
const SCALE: f32 = 2.0;
const WARM: usize = 5;
const ROUNDS: usize = 30;
const VARIANTS: u32 = 9;
const CLEAR: Rgba = Rgba::new(0.06, 0.07, 0.09, 1.0);
const NONE: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);
const INK: Color = Color {
    r: 90,
    g: 200,
    b: 255,
    a: 255,
};

#[derive(Clone, Copy)]
struct Shape {
    name: &'static str,
    cols: u32,
    rows: u32,
    segments: u32,
}

const SHAPES: [Shape; 2] = [
    Shape {
        name: "polylines_1k",
        cols: 40,
        rows: 25,
        segments: 20,
    },
    Shape {
        name: "chart_10k",
        cols: 125,
        rows: 80,
        segments: 6,
    },
];

struct Sample {
    cpu: f64,
    total: f64,
}

impl Shape {
    fn cell(self) -> (f32, f32) {
        (VIEW.0 / self.cols as f32, VIEW.1 / self.rows as f32)
    }

    fn rect(self, index: u32) -> Rect {
        let (w, h) = self.cell();
        let (col, row) = (index % self.cols, index / self.cols);
        Rect::new(Point::new(col as f32 * w, row as f32 * h), w, h)
    }

    fn vector(self, phase: u32) -> CommandScene {
        let theme = Theme::dark();
        let mut scene = SceneBuilder::new();
        for index in 0..self.cols * self.rows {
            let mut node = path(100, 100).move_to(0.0, wave(index, phase));
            for step in 1..=self.segments {
                let x = 100.0 * step as f32 / self.segments as f32;
                node = node.line_to(x, wave(index, step + phase));
            }
            node.stroke(INK, 3.0)
                .draw(&mut scene, self.rect(index), &theme);
        }
        scene.finish()
    }

    fn bitmap(self, vector: &CommandScene) -> CommandScene {
        let width = (VIEW.0 * SCALE) as u32;
        let frame = Frame::new(
            vector,
            VIEW,
            sigil::geometry::PhysicalSize::new(width, (VIEW.1 * SCALE) as u32),
            NONE,
        );
        let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
        let mut scene = SceneBuilder::new();
        for index in 0..self.cols * self.rows {
            let rect = self.rect(index);
            let tile = crop(&rgba, width, rect);
            scene.draw_image(DrawImage {
                node: None,
                source: tile,
                fit: ObjectFit::Fill,
                destination: rect,
                opacity: 10_000,
                crop: None,
                raster: None,
            });
        }
        scene.finish()
    }
}

fn wave(index: u32, step: u32) -> f32 {
    let phase = (index.wrapping_mul(2_654_435_761) % 1000) as f32 / 159.0;
    40.0f32.mul_add((step as f32).mul_add(0.7, phase).sin(), 50.0)
}

fn crop(rgba: &[u8], stride: u32, rect: Rect) -> ImageSource {
    let x0 = (rect.origin.x * SCALE) as u32;
    let y0 = (rect.origin.y * SCALE) as u32;
    let w = ((rect.width * SCALE).ceil() as u32).max(1);
    let h = ((rect.height * SCALE).ceil() as u32).max(1);
    let mut tile = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y0 + h {
        let start = ((y * stride + x0) * 4) as usize;
        tile.extend_from_slice(rgba.get(start..start + (w * 4) as usize).unwrap_or(&[]));
    }
    tile.resize((w * h * 4) as usize, 0);
    ImageSource::from_rgba8(w, h, tile).expect("tile rgba")
}

fn frame(renderer: &mut WgpuRenderer, scene: &CommandScene) -> Sample {
    let (device, queue) = shared_device().expect("sigil-kittest").expect("adapter");
    let capture = Capture::new(VIEW, [(VIEW.0 * SCALE) as u32, (VIEW.1 * SCALE) as u32])
        .scale(SCALE)
        .clear(CLEAR);
    let start = Instant::now();
    let texture = render_capture(renderer, (device, queue), scene, &capture).expect("render");
    let cpu = start.elapsed().as_secs_f64() * 1e3;
    let [w, h] = capture.size_px();
    let pixels = read_pixels(device, queue, &texture, w, h).expect("readback");
    std::hint::black_box(pixels);
    Sample {
        cpu,
        total: start.elapsed().as_secs_f64() * 1e3,
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(f64::NAN)
}

fn interval(diffs: &[f64]) -> (f64, f64) {
    let mut seed = 0x9e37_79b9_u64;
    let mut medians: Vec<f64> = (0..1000)
        .map(|_| {
            let mut pick: Vec<f64> = diffs
                .iter()
                .map(|_| {
                    seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    diffs[(seed >> 33) as usize % diffs.len()]
                })
                .collect();
            median(&mut pick)
        })
        .collect();
    medians.sort_by(f64::total_cmp);
    (medians[25], medians[974])
}

fn report(name: &str, list: &[Sample]) {
    let mut total: Vec<f64> = list.iter().map(|s| s.total).collect();
    let mut cpu: Vec<f64> = list.iter().map(|s| s.cpu).collect();
    println!(
        "  {name:<8} frame {:7.2} ms  encode {:6.2} ms",
        median(&mut total),
        median(&mut cpu)
    );
}

fn run(renderer: &mut WgpuRenderer, shape: Shape) {
    let vector = shape.vector(0);
    let scenes = [
        (
            "floor",
            vec![CommandScene {
                commands: Vec::new(),
            }],
        ),
        ("bitmap", vec![shape.bitmap(&vector)]),
        ("vector", vec![vector]),
        (
            "changing",
            (1..=VARIANTS).map(|phase| shape.vector(phase)).collect(),
        ),
    ];
    let paths = scenes[2].1[0]
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::Path(_)))
        .count();
    let mut samples: Vec<Vec<Sample>> = scenes.iter().map(|_| Vec::new()).collect();
    for round in 0..WARM + ROUNDS {
        for offset in 0..scenes.len() {
            let pick = (round + offset) % scenes.len();
            let variants = &scenes[pick].1;
            let sample = frame(renderer, &variants[round % variants.len()]);
            if round >= WARM {
                samples[pick].push(sample);
            }
        }
    }
    println!(
        "{} ({paths} paths, {}x{} at {SCALE}x)",
        shape.name, VIEW.0, VIEW.1
    );
    for ((name, _), list) in scenes.iter().zip(&samples) {
        report(name, list);
    }
    let mut diffs: Vec<f64> = samples[2]
        .iter()
        .zip(&samples[1])
        .map(|(v, b)| v.total - b.total)
        .collect();
    let (low, high) = interval(&diffs);
    println!(
        "  vector - bitmap: median {:+.2} ms, 95% [{low:+.2}, {high:+.2}]",
        median(&mut diffs)
    );
}

fn main() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        println!("no adapter: skipping renderer_vector_paths");
        return;
    };
    let only = std::env::args().nth(1).filter(|arg| !arg.starts_with('-'));
    for shape in SHAPES {
        if only.as_deref().is_none_or(|name| shape.name.contains(name)) {
            run(&mut renderer, shape);
        }
    }
}
