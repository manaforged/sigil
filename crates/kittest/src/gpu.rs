use std::sync::OnceLock;

use sigil::geometry::PhysicalSize;
pub use sigil::renderer::capture::FORMAT;
use sigil::renderer::capture::{
    CaptureExtent, ImageRenderer, ImageTarget, capture_texture, read_texture, render_to_image,
};
use sigil::renderer::{OffscreenTarget, RendererError, WgpuRenderer};
use sigil::scene::CommandScene;
use sigil::style::Color;

use crate::Error;
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
    const SOFTWARE: [(&str, &str); 5] = [
        ("warp", "warp"),
        ("basic render", "warp"),
        ("llvmpipe", "lavapipe"),
        ("lavapipe", "lavapipe"),
        ("swiftshader", "swiftshader"),
    ];
    let name = info.name.to_ascii_lowercase();
    if let Some(&(_, class)) = SOFTWARE.iter().find(|(needle, _)| name.contains(needle)) {
        return class;
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
    renderer: &mut WgpuRenderer,
    scene: &CommandScene,
    viewport: (f32, f32),
    [width, height]: [u32; 2],
) -> Result<image::RgbaImage, Error> {
    render_to_image(
        ImageRenderer::Gpu(renderer),
        scene,
        ImageTarget {
            size: PhysicalSize::new(width, height),
            viewport,
            clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
        },
    )
    .map_err(|err| Error::Render(err.to_string()))
}

#[must_use]
pub fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    capture_texture(device, CaptureExtent { width, height })
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

pub fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Error> {
    read_texture(device, queue, texture, CaptureExtent { width, height })
        .map_err(|err| Error::Readback(err.to_string()))
}
