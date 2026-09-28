#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::print_stdout)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use skrifa as _;
use smallvec as _;
use softbuffer as _;
use tiny_skia as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;
use zeno as _;

use std::time::Instant;

use sigil::renderer::{ScreenSubmission, TextTuning, WgpuRenderer, set_text_tuning};
use sigil::scene::{CommandScene, SceneBuilder, shape_glyph_run};
use sigil::style::{Rgba8, TextLineHeight, TextStyle};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const LOGICAL_W: f32 = 480.0;
const LOGICAL_H: f32 = 2000.0;
const FONT_SIZE: f32 = 14.0;
const ORIGIN: (f32, f32) = (8.0, 8.0);
const ITERATIONS: usize = 100;
const TARGET_GLYPHS: usize = 2000;
const PARAGRAPH_UNIT: &str = "The quick brown fox jumps over the lazy dog. ";

#[test]
#[ignore = "manual: throughput bench; run with `cargo test -p sigil-ui \
            --test renderer_raster_throughput_bench -- --ignored --nocapture`"]
fn raster_vs_vector_throughput() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        println!("no adapter — skipping raster_vs_vector_throughput");
        return;
    };

    let content = build_paragraph(TARGET_GLYPHS);
    let (scene, glyph_count) = build_scene(&content);
    println!("throughput bench: N={glyph_count} glyphs, iterations={ITERATIONS}");

    let vector_us = bench_path(&mut renderer, &scene, false, ITERATIONS);
    let raster_us = bench_path(&mut renderer, &scene, true, ITERATIONS);

    report("vector", &vector_us);
    report_raster(&raster_us);

    for (label, samples) in [("vector", &vector_us), ("raster", &raster_us)] {
        assert_eq!(samples.len(), ITERATIONS, "{label}: missing samples");
        assert!(
            samples.iter().all(|&us| us.is_finite() && us > 0.0),
            "{label}: non-finite or zero frame timing recorded"
        );
    }
}

fn bench_path(
    renderer: &mut WgpuRenderer,
    scene: &CommandScene,
    raster: bool,
    iterations: usize,
) -> Vec<f64> {
    set_text_tuning(TextTuning {
        raster,
        ..TextTuning::default()
    });
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();
    let phys_w = LOGICAL_W.round() as u32;
    let phys_h = LOGICAL_H.round() as u32;
    let texture = create_render_texture(&device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        renderer
            .submit_screen(
                &ScreenSubmission {
                    device: &device,
                    queue: &queue,
                    view: &view,
                    format: FORMAT,
                    viewport: (LOGICAL_W, LOGICAL_H),
                    scale: 1.0,
                    clear: Some(sigil::hal::Color::BLACK),
                },
                scene,
            )
            .expect("submit_screen");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll");
        samples.push(start.elapsed().as_secs_f64() * 1_000_000.0);
    }
    set_text_tuning(TextTuning::default());
    samples
}

fn build_paragraph(min_len: usize) -> String {
    let mut s = String::with_capacity(min_len + PARAGRAPH_UNIT.len());
    while s.len() < min_len {
        s.push_str(PARAGRAPH_UNIT);
    }
    s
}

fn build_scene(content: &str) -> (CommandScene, usize) {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
        ..TextStyle::default()
    };
    let rect = sigil::hal::Rect::new(
        sigil::hal::Point::new(ORIGIN.0, ORIGIN.1),
        LOGICAL_W - ORIGIN.0,
        LOGICAL_H - ORIGIN.1,
    );
    let run = shape_glyph_run(
        rect,
        content.to_string(),
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        style,
        10_000,
    );
    let glyph_count = run.positioned_glyphs().len();
    let mut builder = SceneBuilder::new();
    builder.draw_glyph_run(run);
    (builder.finish(), glyph_count)
}

fn median_us(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])
    }
}

fn report(label: &str, samples: &[f64]) {
    println!(
        "{label}: median={:.1}us/frame min={:.1} max={:.1}",
        median_us(samples),
        samples.iter().copied().fold(f64::INFINITY, f64::min),
        samples.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
}

fn report_raster(samples: &[f64]) {
    println!("raster cold (iter 1): {:.1}us/frame", samples[0]);
    report("raster warm (iter 2..N)", &samples[1..]);
}

fn create_render_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raster-throughput-bench target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}
