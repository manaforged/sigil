use std::sync::OnceLock;

use sigil::geometry::PhysicalSize;
pub use sigil::renderer::capture::FORMAT;
use sigil::renderer::capture::{
    BYTES_PER_PIXEL, CaptureExtent, ImageRenderer, ImageTarget, capture_texture, read_texture,
    render_to_image,
};
use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil::scene::CommandScene;
use sigil::style::Rgba;

use crate::{Error, ErrorKind};

pub(crate) const CLEAR: Rgba = Rgba::new(0.0, 0.0, 0.0, 1.0);

/// A `wgpu` device and its queue.
pub type Gpu = (wgpu::Device, wgpu::Queue);

static SHARED: OnceLock<Result<(wgpu::Adapter, Gpu), Error>> = OnceLock::new();

/// Reports whether the `SIGIL_ALLOW_NO_GPU` environment variable is `1`, `true`, `yes`, or `on`, which turns a missing adapter into `Ok(None)` instead of an error.
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
        Err(err) if err.is_no_adapter() && no_gpu_allowed() => Ok(None),
        Err(err) => Err(err.clone()),
    }
}

/// Returns the process-wide headless device and queue, opened on first use; `Ok(None)` means no adapter exists and `SIGIL_ALLOW_NO_GPU` is set.
pub fn shared_device() -> Result<Option<&'static Gpu>, Error> {
    Ok(shared()?.map(|(_, gpu)| gpu))
}

/// Names the shared adapter as its backend and vendor class, such as `metal-apple`; `None` means the shared device is not available.
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

/// Creates a renderer attached to the shared headless device; `Ok(None)` means no adapter exists and `SIGIL_ALLOW_NO_GPU` is set.
pub fn headless_renderer() -> Result<Option<WgpuRenderer>, Error> {
    let Some((adapter, (device, queue))) = shared()? else {
        return Ok(None);
    };
    let mut renderer = WgpuRenderer::new();
    renderer.attach_headless_for_tests(adapter, device, queue);
    Ok(Some(renderer))
}

fn open_device() -> Result<(wgpu::Adapter, Gpu), Error> {
    match WgpuRenderer::headless_device_for_tests() {
        Ok((adapter, device, queue)) => Ok((adapter, (device, queue))),
        Err(err) if err.kind() == sigil::ErrorKind::NoAdapter => {
            Err(Error::caused(ErrorKind::NoAdapter, err))
        }
        Err(err) => Err(Error::caused(ErrorKind::Device, err)),
    }
}

/// Target of a raw offscreen capture.
#[must_use]
#[derive(Clone, Copy, Debug)]
pub struct Capture {
    viewport: (f32, f32),
    size_px: [u32; 2],
    scale: f32,
    clear: Rgba,
}

impl Capture {
    /// Creates a capture of a logical viewport into `size_px` pixels, at scale 1 on opaque black.
    pub const fn new(viewport: (f32, f32), size_px: [u32; 2]) -> Self {
        Self {
            viewport,
            size_px,
            scale: 1.0,
            clear: CLEAR,
        }
    }

    /// Sets the device scale of the capture; the default is 1.
    pub const fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Sets the clear color of the capture; the default is opaque black.
    pub const fn clear(mut self, clear: Rgba) -> Self {
        self.clear = clear;
        self
    }

    /// The size of the captured texture in pixels.
    #[must_use]
    pub const fn size_px(&self) -> [u32; 2] {
        self.size_px
    }
}

/// Draws `scene` into a new texture of the size, scale, and clear color in `capture`; `Err` has kind `ErrorKind::Render`.
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
        .map_err(|err| Error::caused(ErrorKind::Render, err))?;
    Ok(texture)
}

/// Draws `scene` on the GPU into a `width` by `height` pixel image cleared to opaque black, with the device scale set to `width` divided by the viewport width.
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
            clear: CLEAR,
        },
    )
    .map_err(|err| Error::caused(ErrorKind::Render, err))
}

/// Creates a `width` by `height` pixel texture in `FORMAT` that can be rendered to and copied from.
#[must_use]
pub fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    capture_texture(device, CaptureExtent { width, height })
}

/// Returns the four bytes of the pixel at (`x`, `y`) in a tightly packed readback `width` pixels wide, and panics when the pixel is outside `pixels`.
#[must_use]
pub fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let start =
        usize::try_from((y * width + x) * BYTES_PER_PIXEL).expect("pixel offset fits usize");
    pixels
        .get(start..start + 4)
        .and_then(|px| <[u8; 4]>::try_from(px).ok())
        .expect("pixel inside the readback")
}

/// Copies `texture` back from the GPU as tightly packed rows of four bytes per pixel; `Err` has kind `ErrorKind::Readback`.
pub fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Error> {
    read_texture(device, queue, texture, CaptureExtent { width, height })
        .map_err(|err| Error::text(ErrorKind::Readback, err))
}
