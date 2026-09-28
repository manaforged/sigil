#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::print_stdout)]

use std::path::PathBuf;

use sigil::hal::{Color, PhysicalSize};
use sigil::renderer::{CpuRenderer, OffscreenTarget, WgpuRenderer, scenes};
use sigil::scene::CommandScene;
use sigil_kittest::gpu::read_pixels;
use sigil_kittest::parity::{CLOSE_TOLERANCE, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, diff};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SCALE: f32 = 2.0;
const LIMITS: [(&str, f32, f32); 12] = [
    ("gradient", 2.69, 0.98),
    ("stroke", 2.87, 0.971),
    ("corner_radius", 1.02, 0.98),
    ("shadow", 1.05, 0.98),
    ("inner_shadow", 1.74, 0.98),
    ("pattern", 1.82, 0.98),
    ("sprite", 1.79, 0.979),
    ("icon", 1.23, 0.98),
    ("decoration", 2.1, 0.967),
    ("glow", 1.27, 0.98),
    ("text", 1.61, 0.977),
    ("translucent", 2.87, 0.98),
];

type Scene = (CommandScene, (f32, f32), Color);

fn catalog() -> Vec<(&'static str, Scene)> {
    vec![
        ("gradient", scenes::gradient()),
        ("stroke", scenes::stroke()),
        ("corner_radius", scenes::corner_radius()),
        ("shadow", scenes::shadow()),
        ("inner_shadow", scenes::inner_shadow()),
        ("pattern", scenes::pattern()),
        ("sprite", scenes::sprite()),
        ("icon", scenes::icon()),
        ("decoration", scenes::decoration()),
        ("glow", scenes::glow()),
        ("text", scenes::text()),
        ("translucent", scenes::translucent()),
    ]
}

fn srgb(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055_f32.mul_add(linear.powf(1.0 / 2.4), -0.055)
    }
}

fn physical(viewport: (f32, f32)) -> PhysicalSize {
    PhysicalSize::new(
        (viewport.0 * SCALE).round() as u32,
        (viewport.1 * SCALE).round() as u32,
    )
}

fn out_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("SIGIL_PARITY_OUT")?;
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn save(dir: Option<&PathBuf>, name: &str, rgba: &[u8], size: PhysicalSize) {
    let Some(dir) = dir else { return };
    image::save_buffer(
        dir.join(format!("{name}.png")),
        rgba,
        size.width,
        size.height,
        image::ColorType::Rgba8,
    )
    .expect("write parity png");
}

fn cpu_pixels(scene: &Scene) -> (Vec<u8>, PhysicalSize) {
    let (commands, viewport, clear) = scene;
    let size = physical(*viewport);
    let pixmap = CpuRenderer::new()
        .render(commands, *clear, *viewport, size)
        .expect("cpu render");
    let mut rgba = Vec::with_capacity(pixmap.pixels().len() * 4);
    for px in pixmap.pixels() {
        let c = px.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    (rgba, size)
}

#[test]
fn cpu_renders_every_catalog_scene_with_ink() {
    let dir = out_dir();
    for (name, scene) in catalog() {
        let (rgba, size) = cpu_pixels(&scene);
        save(dir.as_ref(), &format!("{name}_cpu"), &rgba, size);
        let clear = scene.2;
        let bg = [clear.r, clear.g, clear.b].map(|c| (srgb(c) * 255.0).round() as u8);
        let (pixels, _) = rgba.as_chunks::<4>();
        let ink = pixels
            .iter()
            .filter(|px| px.iter().zip(bg).any(|(&c, b)| c.abs_diff(b) > 8))
            .count();
        assert!(
            ink > 100,
            "{name}: cpu backend painted only {ink} non-background pixels"
        );
    }
}

fn gpu_pixels(renderer: &mut WgpuRenderer, scene: &Scene) -> (Vec<u8>, PhysicalSize) {
    let (commands, viewport, clear) = scene;
    let size = physical(*viewport);
    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cpu-parity capture"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: *viewport,
                scale: SCALE,
            },
            commands,
            *clear,
        )
        .expect("gpu render");
    (
        read_pixels(&device, &queue, &texture, size.width, size.height).expect("sigil-kittest"),
        size,
    )
}

fn limits(name: &str) -> (f32, f32) {
    LIMITS
        .iter()
        .find(|(scene, _, _)| *scene == name)
        .map_or((MAX_MEAN_DIFF, MIN_CLOSE_FRACTION), |&(_, mean, close)| {
            (mean, close)
        })
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware"]
fn cpu_matches_gpu_on_catalog_scenes() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        println!("no adapter; skipping cpu parity");
        return;
    };
    let dir = out_dir();
    let mut failures = Vec::new();
    for (name, scene) in catalog() {
        let (gpu, size) = gpu_pixels(&mut renderer, &scene);
        let (cpu, _) = cpu_pixels(&scene);
        save(dir.as_ref(), &format!("{name}_gpu"), &gpu, size);
        save(dir.as_ref(), &format!("{name}_cpu"), &cpu, size);
        let d = diff(&gpu, &cpu);
        println!(
            "{name}: mean channel diff {:.2}/255, {:.1}% pixels within {CLOSE_TOLERANCE}",
            d.mean,
            d.close * 100.0
        );
        let (max_mean, min_close) = limits(name);
        if d.mean > max_mean || d.close < min_close {
            failures.push(name);
        }
    }
    assert!(
        failures.is_empty(),
        "cpu/gpu parity failed for {failures:?}"
    );
}
