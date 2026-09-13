#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::print_stdout)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use pollster::FutureExt;
use sigil::hal::Color;
use sigil::renderer::{
    CommandScene, CornerRadii, GradientStop, LayerEffect, LayerScope, LinearGradient,
    OffscreenTarget, SceneBuilder, SceneGradientStyle, SceneRectStyle, WgpuRenderer,
};
use sigil::style::Rgba8;
use std::hint::black_box;
use wgpu::TextureFormat;

const FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
const SIGMA_SWEEP: [f32; 5] = [0.0, 6.0, 12.0, 18.0, 24.0];
const COUNT_SWEEP: [u32; 3] = [1, 3, 8];
const SIZE_SWEEP: [u32; 4] = [128, 256, 512, 1024];
const FIXED_SIGMA: f32 = 12.0;
const CLEAR: Color = Color::rgba(0.05, 0.06, 0.09, 1.0);

fn blur_sigma_sweep(c: &mut Criterion) {
    let Some((device, queue)) = try_request_device() else {
        eprintln!("no adapter — skipping blur_cost");
        return;
    };

    let mut group = c.benchmark_group("blur_sigma_sweep");
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(10));

    for sigma in SIGMA_SWEEP {
        let (scene, viewport) = single_blur_scene(240, 240, sigma);
        let target_tex = make_texture(&device, viewport.0, viewport.1);
        let view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");

        group.bench_with_input(BenchmarkId::from_parameter(sigma), &sigma, |b, _| {
            b.iter(|| {
                renderer
                    .render_command_scene_offscreen(
                        OffscreenTarget {
                            device: &device,
                            queue: &queue,
                            view: &view,
                            format: FORMAT,
                            viewport: (viewport.0 as f32, viewport.1 as f32),
                            scale: 1.0,
                        },
                        black_box(&scene),
                        CLEAR,
                    )
                    .expect("render_command_scene_offscreen");
                queue.submit([]);
                drop(device.poll(wgpu::PollType::Poll));
            });
        });
    }

    group.finish();
}

fn blur_count_sweep(c: &mut Criterion) {
    let Some((device, queue)) = try_request_device() else {
        eprintln!("no adapter — skipping blur_cost");
        return;
    };

    let mut group = c.benchmark_group("blur_count_sweep");
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(10));

    for count in COUNT_SWEEP {
        let (scene, viewport) = multi_blur_scene(count, 240, 240, FIXED_SIGMA);
        let target_tex = make_texture(&device, viewport.0, viewport.1);
        let view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                renderer
                    .render_command_scene_offscreen(
                        OffscreenTarget {
                            device: &device,
                            queue: &queue,
                            view: &view,
                            format: FORMAT,
                            viewport: (viewport.0 as f32, viewport.1 as f32),
                            scale: 1.0,
                        },
                        black_box(&scene),
                        CLEAR,
                    )
                    .expect("render_command_scene_offscreen");
                queue.submit([]);
                drop(device.poll(wgpu::PollType::Poll));
            });
        });
    }

    group.finish();
}

fn blur_size_sweep(c: &mut Criterion) {
    let Some((device, queue)) = try_request_device() else {
        eprintln!("no adapter — skipping blur_cost");
        return;
    };

    let mut group = c.benchmark_group("blur_size_sweep");
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(10));

    for side in SIZE_SWEEP {
        let (scene, viewport) = single_blur_scene(side, side, FIXED_SIGMA);
        let target_tex = make_texture(&device, viewport.0, viewport.1);
        let view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");

        group.bench_with_input(BenchmarkId::from_parameter(side), &side, |b, _| {
            b.iter(|| {
                renderer
                    .render_command_scene_offscreen(
                        OffscreenTarget {
                            device: &device,
                            queue: &queue,
                            view: &view,
                            format: FORMAT,
                            viewport: (viewport.0 as f32, viewport.1 as f32),
                            scale: 1.0,
                        },
                        black_box(&scene),
                        CLEAR,
                    )
                    .expect("render_command_scene_offscreen");
                queue.submit([]);
                drop(device.poll(wgpu::PollType::Poll));
            });
        });
    }

    group.finish();
}

fn single_blur_scene(w: u32, h: u32, sigma: f32) -> (CommandScene, (u32, u32)) {
    let vp_w = w + 16;
    let vp_h = h + 16;
    let mut scene = SceneBuilder::new();
    draw_gradient_background(&mut scene, vp_w, vp_h);
    push_blur_surface(&mut scene, 8.0, 8.0, w as f32, h as f32, sigma);
    (scene.finish(), (vp_w, vp_h))
}

fn multi_blur_scene(
    count: u32,
    surf_w: u32,
    surf_h: u32,
    sigma: f32,
) -> (CommandScene, (u32, u32)) {
    let gap = 8_u32;
    let vp_w = count * surf_w + (count + 1) * gap;
    let vp_h = surf_h + 2 * gap;
    let mut scene = SceneBuilder::new();
    draw_gradient_background(&mut scene, vp_w, vp_h);
    for i in 0..count {
        let x = (gap + i * (surf_w + gap)) as f32;
        let y = gap as f32;
        push_blur_surface(&mut scene, x, y, surf_w as f32, surf_h as f32, sigma);
    }
    (scene.finish(), (vp_w, vp_h))
}

fn draw_gradient_background(scene: &mut SceneBuilder, w: u32, h: u32) {
    use sigil::hal::{Point, Rect};
    scene.draw_gradient(
        Rect::new(Point::new(0.0, 0.0), w as f32, h as f32),
        SceneGradientStyle::new(
            LinearGradient::new(
                Point::new(0.0, 0.0),
                Point::new(w as f32, 0.0),
                vec![
                    GradientStop::new(0.0, rgba(220, 50, 50, 255)),
                    GradientStop::new(1.0, rgba(50, 200, 220, 255)),
                ],
            ),
            CornerRadii::ZERO,
            10_000,
        ),
    );
}

#[allow(clippy::too_many_arguments, reason = "bench harness; deliberate")]
fn push_blur_surface(scene: &mut SceneBuilder, x: f32, y: f32, w: f32, h: f32, sigma: f32) {
    use sigil::hal::{Point, Rect};
    let clip = Rect::new(Point::new(x, y), w, h);
    scene.push_layer(LayerScope {
        clip,
        clip_radii: CornerRadii::ZERO,
        effect: blur_effect(sigma),
        opacity: 1.0,
    });
    scene.draw_rect(
        clip,
        SceneRectStyle::new(rgba(240, 240, 250, 220), CornerRadii::uniform(12.0), 10_000),
    );
    scene.pop_layer();
}

#[allow(clippy::missing_const_for_fn, reason = "bench harness; deliberate")]
fn blur_effect(_sigma: f32) -> LayerEffect {
    LayerEffect::Identity
}

fn try_request_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
        .ok()?;
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("blur-cost-bench device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()
}

fn make_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blur-cost-bench target"),
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

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

criterion_group!(benches, blur_sigma_sweep, blur_count_sweep, blur_size_sweep);
criterion_main!(benches);
