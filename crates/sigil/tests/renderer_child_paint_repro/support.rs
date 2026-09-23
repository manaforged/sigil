#![allow(clippy::cast_lossless, clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss, clippy::indexing_slicing)]
#![allow(clippy::unwrap_used)]

pub(super) use sigil_kittest::gpu::read_pixels;

use pollster::FutureExt as _;

use sigil::hal::Size;
use sigil::kernel::{Cx, TextContent};
use sigil::layout::{LayoutEngine, TextMeasure};
use sigil::renderer::{OffscreenTarget, WgpuRenderer, emit_scene};

pub(super) struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _source: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

pub(super) fn close(r: u8, g: u8, b: u8, expected: sigil::style::Rgba8, tolerance: i32) -> bool {
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
    background: sigil::style::Rgba8,
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
    build: impl FnOnce(&mut Cx) -> sigil::kernel::NodeId,
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
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
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

fn render_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &sigil::renderer::CommandScene,
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
    let clear = sigil::hal::Color {
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
