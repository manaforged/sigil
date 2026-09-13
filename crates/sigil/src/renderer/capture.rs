use std::error::Error;
use std::io;
use std::sync::mpsc;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const BYTES_PER_PIXEL: u32 = 4;
const MIN_LONG_AXIS: f32 = 1024.0;
const MAX_LONG_AXIS: f32 = 2048.0;
const MAX_INTEGER_SCALE: f32 = 4.0;

#[must_use]
pub fn capture_texture(device: &wgpu::Device, extent: CaptureExtent) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sigil-renderer capture texture"),
        size: wgpu::Extent3d {
            width: extent.width,
            height: extent.height,
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

pub fn clear_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    color: crate::hal::Color,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sigil-renderer capture split pre-clear"),
    });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer capture split pre-clear pass"),
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
    queue.submit(core::iter::once(encoder.finish()));
}

pub fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    extent: CaptureExtent,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let layout = ReadbackLayout::new(extent);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sigil-renderer capture readback"),
        size: u64::from(layout.padded_bytes_per_row) * u64::from(extent.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    copy_texture_to_buffer(device, queue, texture, &buffer, layout);
    map_readback(device, &buffer, layout)
}

fn copy_texture_to_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    layout: ReadbackLayout,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sigil-renderer capture copy encoder"),
    });
    encoder.copy_texture_to_buffer(
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
                bytes_per_row: Some(layout.padded_bytes_per_row),
                rows_per_image: Some(layout.extent.height),
            },
        },
        wgpu::Extent3d {
            width: layout.extent.width,
            height: layout.extent.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(encoder.finish()));
}

fn map_readback(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    layout: ReadbackLayout,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        drop(tx.send(result));
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()
        .map_err(|err| io::Error::other(format!("readback map callback dropped: {err}")))??;
    let mapped = slice.get_mapped_range()?;
    let pixels = strip_padding(&mapped, layout);
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}

fn strip_padding(mapped: &[u8], layout: ReadbackLayout) -> Vec<u8> {
    let unpadded = layout.unpadded_bytes_per_row as usize;
    let padded = layout.padded_bytes_per_row as usize;
    let height = layout.extent.height as usize;
    let mut pixels = vec![0; unpadded * height];
    for row in 0..height {
        let src = row * padded;
        let dst = row * unpadded;
        #[allow(
            clippy::indexing_slicing,
            reason = "ranges derive from ReadbackLayout, which sizes both buffers"
        )]
        pixels[dst..dst + unpadded].copy_from_slice(&mapped[src..src + unpadded]);
    }
    pixels
}

#[derive(Clone, Copy)]
struct ReadbackLayout {
    extent: CaptureExtent,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
}

impl ReadbackLayout {
    const fn new(extent: CaptureExtent) -> Self {
        let unpadded_bytes_per_row = extent.width * BYTES_PER_PIXEL;
        Self {
            extent,
            unpadded_bytes_per_row,
            padded_bytes_per_row: align_to(
                unpadded_bytes_per_row,
                wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
            ),
        }
    }
}

#[derive(Clone, Copy)]
pub struct CaptureExtent {
    pub width: u32,
    pub height: u32,
}

impl CaptureExtent {
    #[must_use]
    pub fn from_viewport(viewport: (f32, f32)) -> Self {
        let logical_width = viewport.0.max(1.0);
        let logical_height = viewport.1.max(1.0);
        let long_axis = logical_width.max(logical_height);
        let integer_scale = (MAX_LONG_AXIS / long_axis)
            .ceil()
            .clamp(1.0, MAX_INTEGER_SCALE);
        let scaled_width = logical_width * integer_scale;
        let scaled_height = logical_height * integer_scale;
        let scaled_long_axis = scaled_width.max(scaled_height);
        let clamped_long = scaled_long_axis.clamp(MIN_LONG_AXIS, MAX_LONG_AXIS);
        let clamp_scale = clamped_long / scaled_long_axis;
        Self {
            width: rounded_extent(scaled_width * clamp_scale),
            height: rounded_extent(scaled_height * clamp_scale),
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is rounded and clamped to 1.0..=2048.0 before the cast"
)]
const fn rounded_extent(value: f32) -> u32 {
    value.round().clamp(1.0, MAX_LONG_AXIS) as u32
}

const fn align_to(value: u32, alignment: u32) -> u32 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}
