#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::print_stdout)]

use std::path::PathBuf;

use sigil_hal::{Color, PhysicalSize};
use sigil_renderer::{CommandScene, CpuRenderer, OffscreenTarget, WgpuRenderer, scenes};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SCALE: f32 = 2.0;
const MAX_MEAN_DIFF: f32 = 20.0;
const MIN_CLOSE_FRACTION: f32 = 0.85;
const CLOSE_TOLERANCE: u8 = 48;

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
    (read_pixels(&device, &queue, &texture, size), size)
}

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: PhysicalSize,
) -> Vec<u8> {
    let unpadded = size.width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cpu-parity readback"),
        size: u64::from(padded) * u64::from(size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(size.height),
            },
        },
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(enc.finish()));
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = slice.get_mapped_range().expect("mapped range");
    let mut out = Vec::with_capacity((unpadded * size.height) as usize);
    for row in 0..size.height as usize {
        let start = row * padded as usize;
        out.extend_from_slice(&data[start..start + unpadded as usize]);
    }
    out
}

struct Diff {
    mean: f32,
    close: f32,
}

fn diff(a: &[u8], b: &[u8]) -> Diff {
    let mut total = 0_u64;
    let mut close = 0_usize;
    let pixels = a.len() / 4;
    let (a, _) = a.as_chunks::<4>();
    let (b, _) = b.as_chunks::<4>();
    for (pa, pb) in a.iter().zip(b) {
        let d = [0, 1, 2].map(|i| pa[i].abs_diff(pb[i]));
        total += d.iter().map(|&v| u64::from(v)).sum::<u64>();
        if d.iter().all(|&v| v <= CLOSE_TOLERANCE) {
            close += 1;
        }
    }
    Diff {
        mean: total as f32 / (pixels * 3) as f32,
        close: close as f32 / pixels as f32,
    }
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware"]
fn cpu_matches_gpu_on_catalog_scenes() {
    let mut renderer = WgpuRenderer::new().expect("renderer");
    if renderer.ensure_headless_for_tests().is_err() {
        println!("no adapter; skipping cpu parity");
        return;
    }
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
        if d.mean > MAX_MEAN_DIFF || d.close < MIN_CLOSE_FRACTION {
            failures.push(name);
        }
    }
    assert!(
        failures.is_empty(),
        "cpu/gpu parity failed for {failures:?}"
    );
}
