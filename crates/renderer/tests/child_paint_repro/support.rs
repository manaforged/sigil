#![allow(clippy::cast_lossless, clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss, clippy::indexing_slicing)]
#![allow(clippy::unwrap_used)]

use pollster::FutureExt as _;

use sigil_hal::Size;
use sigil_kernel::{Cx, TextContent};
use sigil_layout::{LayoutEngine, TextMeasure};
use sigil_renderer::{OffscreenTarget, WgpuRenderer, emit_scene};

pub(super) struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _source: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

pub(super) fn close(r: u8, g: u8, b: u8, expected: sigil_style::Rgba8, tolerance: i32) -> bool {
    (i32::from(r) - i32::from(expected.r)).abs() <= tolerance
        && (i32::from(g) - i32::from(expected.g)).abs() <= tolerance
        && (i32::from(b) - i32::from(expected.b)).abs() <= tolerance
}

#[derive(Clone, Copy)]
pub(super) struct InkWindow {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) w: u32,
    pub(super) h: u32,
}

pub(super) fn count_ink(
    img: &image::RgbaImage,
    window: InkWindow,
    background: sigil_style::Rgba8,
) -> u32 {
    let bg_sum = u32::from(background.r) + u32::from(background.g) + u32::from(background.b);
    let threshold = bg_sum.saturating_sub(150);
    let mut ink = 0u32;
    for py in window.y..(window.y + window.h) {
        for px in window.x..(window.x + window.w) {
            let pixel = img.get_pixel(px, py);
            let [channel_r, channel_g, channel_b, _a] = pixel.0;
            let sum = u32::from(channel_r) + u32::from(channel_g) + u32::from(channel_b);
            if sum < threshold {
                ink += 1;
            }
        }
    }
    ink
}

pub(super) fn capture(
    build: impl FnOnce(&mut Cx) -> sigil_kernel::NodeId,
    size_px: [u32; 2],
) -> image::RgbaImage {
    let mut cx = Cx::new_root();
    let root = build(&mut cx);

    let [w, h] = size_px;
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(w as f32, h as f32))
        .expect("layout compute");

    let scene = emit_scene(&cx, root);
    let (device, queue) = request_device();
    render_to_image(&device, &queue, &scene, (w as f32, h as f32), size_px)
}

fn request_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
        .expect("no wgpu adapter — child-paint repro needs a real GPU");
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("child-paint-repro device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .expect("wgpu adapter has no usable device")
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

fn render_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &sigil_renderer::CommandScene,
    viewport: (f32, f32),
    size_px: [u32; 2],
) -> image::RgbaImage {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("child-paint-repro capture"),
        size: wgpu::Extent3d {
            width: size_px[0],
            height: size_px[1],
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
    let clear = sigil_hal::Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport,
                scale: 1.0,
            },
            scene,
            clear,
        )
        .expect("render_command_scene_offscreen");

    let raw = read_pixels(device, queue, &texture, size_px[0], size_px[1]);
    image::RgbaImage::from_raw(size_px[0], size_px[1], raw).expect("rgba image from raw")
}

const fn padded_bytes_per_row(unpadded_bpr: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let remainder = unpadded_bpr % align;
    if remainder == 0 {
        unpadded_bpr
    } else {
        unpadded_bpr + align - remainder
    }
}

fn copy_texture_to_readback_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size_px: [u32; 2],
    padded_bpr: u32,
) -> wgpu::Buffer {
    let [width, height] = size_px;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("child-paint-repro readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("child-paint-repro copy"),
    });
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
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(enc.finish()));
    buffer
}

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let padded_bpr = padded_bytes_per_row(unpadded_bpr);
    let buffer =
        copy_texture_to_readback_buffer(device, queue, texture, [width, height], padded_bpr);

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll");
    rx.recv().expect("map callback").expect("map result");

    let mapped = slice.get_mapped_range().expect("mapped range");
    let un = unpadded_bpr as usize;
    let pad = padded_bpr as usize;
    let h = height as usize;
    let mut out = vec![0u8; un * h];
    for row in 0..h {
        out[row * un..row * un + un].copy_from_slice(&mapped[row * pad..row * pad + un]);
    }
    drop(mapped);
    buffer.unmap();
    out
}
