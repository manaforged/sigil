//! Shared wgpu plumbing for `split_path_pixel_check`.
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::too_many_arguments)]

use std::sync::mpsc;

use pollster::FutureExt as _;
use sigil_hal::Color;

pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub(super) const BYTES_PER_PIXEL: u32 = 4;

pub(super) fn try_request_device(label: &str) -> Option<(wgpu::Device, wgpu::Queue)> {
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
            label: Some(label),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()
}

pub(super) fn create_capture_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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

pub(super) fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("split-path-depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

pub(super) fn pre_clear_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    color: Color,
) {
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("split-path pre-clear"),
    });
    {
        let _p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("split-path pre-clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(color.r),
                        g: f64::from(color.g),
                        b: f64::from(color.b),
                        a: f64::from(color.a),
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    queue.submit(core::iter::once(enc.finish()));
}

pub(super) fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let padded_bpr = padded_bytes_per_row(unpadded_bpr);
    let buffer = readback_buffer(device, padded_bpr, height);
    copy_texture_to_buffer(device, queue, texture, &buffer, width, height, padded_bpr);
    map_readback(device, &buffer, unpadded_bpr, padded_bpr, height)
}

pub(super) fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let b = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    [pixels[b], pixels[b + 1], pixels[b + 2], pixels[b + 3]]
}

pub(super) fn linear_to_srgb_u8(c: f32) -> u8 {
    let srgb = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (srgb.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn padded_bytes_per_row(unpadded_bpr: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let remainder = unpadded_bpr % align;
    if remainder == 0 {
        unpadded_bpr
    } else {
        unpadded_bpr + align - remainder
    }
}

fn readback_buffer(device: &wgpu::Device, padded_bpr: u32, height: u32) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("split-path readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

fn copy_texture_to_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bpr: u32,
) {
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("split-path copy encoder"),
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer,
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
}

fn map_readback(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    unpadded_bpr: u32,
    padded_bpr: u32,
    height: u32,
) -> Vec<u8> {
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
    let out = strip_padding(&mapped, unpadded_bpr, padded_bpr, height);
    drop(mapped);
    buffer.unmap();
    out
}

fn strip_padding(mapped: &[u8], unpadded_bpr: u32, padded_bpr: u32, height: u32) -> Vec<u8> {
    let un = unpadded_bpr as usize;
    let pad = padded_bpr as usize;
    let h = height as usize;
    let mut out = vec![0u8; un * h];
    for row in 0..h {
        out[row * un..row * un + un].copy_from_slice(&mapped[row * pad..row * pad + un]);
    }
    out
}
