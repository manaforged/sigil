use std::error::Error;
use std::io;
use std::sync::mpsc;

use crate::geometry::PhysicalSize;
use crate::renderer::cpu::CpuRenderer;
use crate::renderer::error::RendererError;
use crate::renderer::offscreen::OffscreenTarget;
use crate::renderer::renderer::WgpuRenderer;
use crate::renderer::scene_command::CommandScene;
use crate::style::Color;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const BYTES_PER_PIXEL: u32 = 4;

pub enum ImageRenderer<'a> {
    Gpu(&'a mut WgpuRenderer),
    Cpu(&'a mut CpuRenderer),
}

#[derive(Clone, Copy, Debug)]
pub struct ImageTarget {
    pub size: PhysicalSize,
    pub viewport: (f32, f32),
    pub clear: Color,
}

impl ImageTarget {
    #[allow(
        clippy::cast_precision_loss,
        reason = "image dimensions are far below the f32 mantissa limit"
    )]
    fn scale(self) -> f32 {
        self.size.width as f32 / self.viewport.0.max(1.0)
    }
}

pub fn render_to_image(
    renderer: ImageRenderer<'_>,
    scene: &CommandScene,
    target: ImageTarget,
) -> Result<image::RgbaImage, RendererError> {
    if target.size.width == 0 || target.size.height == 0 {
        return Err(RendererError::InvalidViewport);
    }
    let rgba = match renderer {
        ImageRenderer::Gpu(renderer) => gpu_pixels(renderer, scene, target)?,
        ImageRenderer::Cpu(renderer) => cpu_pixels(renderer, scene, target)?,
    };
    image::RgbaImage::from_raw(target.size.width, target.size.height, rgba)
        .ok_or(RendererError::InvalidViewport)
}

fn gpu_pixels(
    renderer: &mut WgpuRenderer,
    scene: &CommandScene,
    target: ImageTarget,
) -> Result<Vec<u8>, RendererError> {
    let (device, queue) = {
        let ready = renderer.state.ensure_ready_headless()?;
        (ready.device.clone(), ready.queue.clone())
    };
    let extent = CaptureExtent {
        width: target.size.width,
        height: target.size.height,
    };
    let texture = capture_texture(&device, extent);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer.render_command_scene_offscreen(
        OffscreenTarget {
            device: &device,
            queue: &queue,
            view: &view,
            format: FORMAT,
            viewport: target.viewport,
            scale: target.scale(),
        },
        scene,
        target.clear,
    )?;
    read_texture(&device, &queue, &texture, extent)
        .map_err(|err| RendererError::Software(err.to_string()))
}

fn cpu_pixels(
    renderer: &mut CpuRenderer,
    scene: &CommandScene,
    target: ImageTarget,
) -> Result<Vec<u8>, RendererError> {
    let pixmap = renderer.render(scene, target.clear, target.viewport, target.size)?;
    Ok(pixmap
        .pixels()
        .iter()
        .flat_map(|px| {
            let color = px.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect())
}

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
    mapped
        .chunks((layout.padded_bytes_per_row as usize).max(1))
        .take(layout.extent.height as usize)
        .flat_map(|row| row.get(..unpadded).unwrap_or_default())
        .copied()
        .collect()
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
            padded_bytes_per_row: unpadded_bytes_per_row
                .next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
        }
    }
}

#[derive(Clone, Copy)]
pub struct CaptureExtent {
    pub width: u32,
    pub height: u32,
}
