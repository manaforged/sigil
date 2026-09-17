use std::sync::mpsc;

use pollster::FutureExt;
use sigil::hal::Color;
use sigil::renderer::{CommandScene, OffscreenTarget, WgpuRenderer};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

pub(super) fn request_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on();
    let Ok(adapter) = adapter else {
        assert!(
            std::env::var_os("SIGIL_REQUIRE_GOLDENS").is_none(),
            "no wgpu adapter while SIGIL_REQUIRE_GOLDENS is set"
        );
        return None;
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("visual-golden device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .expect("wgpu adapter has no usable device for the visual-golden suite");
    Some((device, queue))
}

pub(super) fn render_scene_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
    viewport: (f32, f32),
    size_px: [u32; 2],
) -> image::RgbaImage {
    let texture = create_capture_texture(device, size_px[0], size_px[1]);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let clear = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    let scale = size_px[0] as f32 / viewport.0;

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport,
                scale,
            },
            scene,
            clear,
        )
        .expect("render_command_scene_offscreen");

    let raw = read_pixels(device, queue, &texture, size_px[0], size_px[1]);
    image::RgbaImage::from_raw(size_px[0], size_px[1], raw).expect("rgba image from raw")
}

fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("visual-golden capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bpr = {
        let r = unpadded_bpr % align;
        if r == 0 {
            unpadded_bpr
        } else {
            unpadded_bpr + align - r
        }
    };
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("visual-golden readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("visual-golden copy"),
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

    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
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
