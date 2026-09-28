use std::sync::{OnceLock, mpsc};

use sigil::hal::Color;
use sigil::renderer::{OffscreenTarget, RendererError, WgpuRenderer};
use sigil::scene::CommandScene;

use crate::{Error, extent_f32};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

pub type Gpu = (wgpu::Device, wgpu::Queue);

static SHARED: OnceLock<Result<(wgpu::Adapter, Gpu), Error>> = OnceLock::new();

#[must_use]
pub fn no_gpu_allowed() -> bool {
    matches!(
        std::env::var("SIGIL_ALLOW_NO_GPU").ok().as_deref(),
        Some("1" | "true" | "yes" | "on"),
    )
}

fn shared() -> Result<Option<&'static (wgpu::Adapter, Gpu)>, Error> {
    match SHARED.get_or_init(open_device) {
        Ok(shared) => Ok(Some(shared)),
        Err(Error::NoAdapter(_)) if no_gpu_allowed() => Ok(None),
        Err(err) => Err(err.clone()),
    }
}

pub fn shared_device() -> Result<Option<&'static Gpu>, Error> {
    Ok(shared()?.map(|(_, gpu)| gpu))
}

#[must_use]
pub fn adapter_key() -> Option<String> {
    let (adapter, _) = shared().ok()??;
    let info = adapter.get_info();
    Some(format!("{}-{}", info.backend.to_str(), vendor_class(&info)))
}

fn vendor_class(info: &wgpu::AdapterInfo) -> &'static str {
    let name = info.name.to_ascii_lowercase();
    if name.contains("warp") || name.contains("basic render") {
        return "warp";
    }
    if name.contains("llvmpipe") || name.contains("lavapipe") {
        return "lavapipe";
    }
    if name.contains("swiftshader") {
        return "swiftshader";
    }
    if name.starts_with("apple") {
        return "apple";
    }
    match info.vendor & 0xffff {
        0x10de => "nvidia",
        0x1002 | 0x1022 => "amd",
        0x8086 => "intel",
        0x106b => "apple",
        0x1414 => "warp",
        0x5143 => "qualcomm",
        0x13b5 => "arm",
        _ if info.device_type == wgpu::DeviceType::Cpu => "cpu",
        _ => "other",
    }
}

pub fn headless_renderer() -> Result<Option<WgpuRenderer>, Error> {
    let Some((adapter, (device, queue))) = shared()? else {
        return Ok(None);
    };
    let mut renderer = WgpuRenderer::new().map_err(|err| Error::Render(err.to_string()))?;
    renderer.attach_headless_for_tests(adapter, device, queue);
    Ok(Some(renderer))
}

fn open_device() -> Result<(wgpu::Adapter, Gpu), Error> {
    match WgpuRenderer::headless_device_for_tests() {
        Ok((adapter, device, queue)) => Ok((adapter, (device, queue))),
        Err(RendererError::NoAdapter) => {
            Err(Error::NoAdapter(RendererError::NoAdapter.to_string()))
        }
        Err(err) => Err(Error::Device(err.to_string())),
    }
}

pub struct Capture {
    pub viewport: (f32, f32),
    pub size_px: [u32; 2],
    pub scale: f32,
    pub clear: Color,
}

pub fn render_capture(
    renderer: &mut WgpuRenderer,
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &CommandScene,
    capture: &Capture,
) -> Result<wgpu::Texture, Error> {
    let [width, height] = capture.size_px;
    let texture = create_capture_texture(device, width, height);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: capture.viewport,
                scale: capture.scale,
            },
            scene,
            capture.clear,
        )
        .map_err(|err| Error::Render(err.to_string()))?;
    Ok(texture)
}

pub fn render_scene_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
    viewport: (f32, f32),
    size_px: [u32; 2],
) -> Result<image::RgbaImage, Error> {
    let [width, height] = size_px;
    let mut renderer = WgpuRenderer::new().map_err(|err| Error::Render(err.to_string()))?;
    let capture = Capture {
        viewport,
        size_px,
        scale: extent_f32(width) / viewport.0,
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let texture = render_capture(&mut renderer, (device, queue), scene, &capture)?;
    let raw = read_pixels(device, queue, &texture, width, height)?;
    image::RgbaImage::from_raw(width, height, raw)
        .ok_or_else(|| Error::Readback("pixel buffer does not match the image size".to_owned()))
}

#[must_use]
pub fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
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

#[must_use]
pub fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let start =
        usize::try_from((y * width + x) * BYTES_PER_PIXEL).expect("pixel offset fits usize");
    pixels
        .get(start..start + 4)
        .and_then(|px| <[u8; 4]>::try_from(px).ok())
        .expect("pixel inside the readback")
}

fn readback_error(err: impl std::fmt::Display) -> Error {
    Error::Readback(err.to_string())
}

pub fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Error> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let padded_bpr = unpadded_bpr.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let buffer = submit_copy(device, queue, texture, extent, padded_bpr);
    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        drop(tx.send(result));
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(readback_error)?;
    rx.recv().map_err(readback_error)?.map_err(readback_error)?;
    let row_len = usize::try_from(unpadded_bpr).map_err(readback_error)?;
    let stride = usize::try_from(padded_bpr).map_err(readback_error)?;
    let rows = usize::try_from(height).map_err(readback_error)?;
    let mapped = slice.get_mapped_range().map_err(readback_error)?;
    let mut out = Vec::with_capacity(row_len * rows);
    for row in mapped.chunks(stride).take(rows) {
        let pixels = row
            .get(..row_len)
            .ok_or_else(|| readback_error("readback row is shorter than the image"))?;
        out.extend_from_slice(pixels);
    }
    drop(mapped);
    buffer.unmap();
    Ok(out)
}

fn submit_copy(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    extent: wgpu::Extent3d,
    padded_bpr: u32,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("visual-golden readback"),
        size: u64::from(padded_bpr) * u64::from(extent.height),
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
                rows_per_image: Some(extent.height),
            },
        },
        extent,
    );
    queue.submit(core::iter::once(enc.finish()));
    buffer
}
