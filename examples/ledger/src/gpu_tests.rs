#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::time::Instant;

use sigil::prelude::{Host, Runtime};
use sigil_hal::Color;
use sigil_renderer::capture::{CaptureExtent, FORMAT, capture_texture};
use sigil_renderer::{OffscreenTarget, WgpuRenderer};

use crate::App;
use crate::view::{ROW_H, WINDOW_H, WINDOW_W, app_tree};

const SCALE: f32 = 2.0;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ledger gpu frames"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .ok()
}

fn percentiles(mut us: Vec<u128>) -> (u128, u128, u128) {
    us.sort_unstable();
    let n = us.len();
    (us[n / 2], us[n * 95 / 100], us[n - 1])
}

#[test]
#[ignore = "headless gpu frame budget; run with --release -- --ignored --nocapture"]
fn gpu_frames_stay_under_budget() {
    let Some((device, queue)) = device() else {
        println!("no adapter; skipping");
        return;
    };
    sigil::testing::with_runtime("ledger-gpu-frames", |_| {
        let mut renderer = WgpuRenderer::new().unwrap();
        let mut host = Host::new();
        let app = App::new(10_000);
        let tree = app.clone();
        host.build(move |cx| cx.add(app_tree(cx, &tree)));
        let viewport = (WINDOW_W, WINDOW_H);
        let extent = CaptureExtent {
            width: (WINDOW_W * SCALE) as u32,
            height: (WINDOW_H * SCALE) as u32,
        };
        let texture = capture_texture(&device, extent);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut layout = Vec::new();
        let mut emit = Vec::new();
        let mut gpu = Vec::new();
        sigil_profile::enable();
        sigil_profile::reset();
        for frame in 0..120_u32 {
            sigil_profile::begin_frame();
            Runtime::begin_effect();
            app.live.tick();
            if frame % 2 == 0 {
                app.scroll.set(frame as f32 * ROW_H * 1.5);
            }
            Runtime::begin_layout();
            let t0 = Instant::now();
            host.layout(viewport).unwrap();
            let tl = Instant::now();
            let scene = host.scene(viewport).unwrap();
            Runtime::begin_paint();
            let t1 = Instant::now();
            renderer
                .render_command_scene_offscreen(
                    OffscreenTarget {
                        device: &device,
                        queue: &queue,
                        view: &view,
                        format: FORMAT,
                        viewport,
                        scale: SCALE,
                    },
                    &scene,
                    Color::rgba(1.0, 1.0, 1.0, 1.0),
                )
                .unwrap();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let t2 = Instant::now();
            sigil_profile::tick_frame();
            if frame >= 10 {
                layout.push((tl - t0).as_micros());
                emit.push((t1 - tl).as_micros());
                gpu.push((t2 - t1).as_micros());
            }
        }
        let (l50, l95, lmax) = percentiles(layout);
        let (e50, e95, emax) = percentiles(emit);
        let (g50, g95, gmax) = percentiles(gpu);
        println!("{}", sigil_profile::summary());
        println!("layout: p50 {l50} us, p95 {l95} us, max {lmax} us");
        println!("emit: p50 {e50} us, p95 {e95} us, max {emax} us");
        println!("gpu submit+wait: p50 {g50} us, p95 {g95} us, max {gmax} us");
        assert!(g95 < 12_000, "gpu p95 {g95} us over budget");
    });
}

#[test]
#[ignore = "headless cpu frame budget; run with --release -- --ignored --nocapture"]
fn cpu_frames_report_their_cost() {
    use sigil_hal::PhysicalSize;
    use sigil_renderer::CpuRenderer;
    sigil::testing::with_runtime("ledger-cpu-frames", |_| {
        let mut renderer = CpuRenderer::new();
        let mut host = Host::new();
        let app = App::new(10_000);
        let tree = app.clone();
        host.build(move |cx| cx.add(app_tree(cx, &tree)));
        let viewport = (WINDOW_W, WINDOW_H);
        let size = PhysicalSize::new((WINDOW_W * SCALE) as u32, (WINDOW_H * SCALE) as u32);
        let mut raster = Vec::new();
        for frame in 0..60_u32 {
            Runtime::begin_effect();
            app.live.tick();
            if frame % 2 == 0 {
                app.scroll.set(frame as f32 * ROW_H * 1.5);
            }
            Runtime::begin_layout();
            let scene = host.scene(viewport).unwrap();
            Runtime::begin_paint();
            let t0 = Instant::now();
            let pixmap = renderer
                .render(&scene, Color::rgba(1.0, 1.0, 1.0, 1.0), viewport, size)
                .unwrap();
            if frame >= 5 {
                raster.push(t0.elapsed().as_micros());
            }
            if frame == 30
                && let Some(dir) = std::env::var_os("SIGIL_PARITY_OUT")
            {
                std::fs::create_dir_all(&dir).unwrap();
                let path = std::path::Path::new(&dir).join("ledger_cpu.png");
                pixmap.save_png(&path).unwrap();
            }
        }
        let (r50, r95, rmax) = percentiles(raster);
        println!(
            "cpu raster {}x{}: p50 {r50} us, p95 {r95} us, max {rmax} us",
            size.width, size.height
        );
    });
}
