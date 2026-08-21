#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Headless PNG capture for renderer phase showcase scenes.

#![allow(clippy::indexing_slicing)] // Examples may slice staging buffers; see the internal hygiene-cleanup-framework plan.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::print_stdout)]
#![allow(missing_docs)]

#[cfg(feature = "capture")]
use std::{error::Error, io, path::Path};

#[cfg(feature = "capture")]
use image::{ColorType, ImageFormat};
#[cfg(feature = "capture")]
use sigil_renderer::{CommandScene, ScreenSubmission, WgpuRenderer, scenes};

#[cfg(feature = "capture")]
const SPLIT_WIDTH: u32 = 1280;
#[cfg(feature = "capture")]
const SPLIT_HALF_HEIGHT: u32 = 400;
#[cfg(feature = "capture")]
const SPLIT_HEIGHT: u32 = SPLIT_HALF_HEIGHT * 2;
#[cfg(feature = "capture")]
const SPLIT_VIEWPORT: (f32, f32) = (SPLIT_WIDTH as f32, SPLIT_HALF_HEIGHT as f32);

#[cfg(feature = "capture")]
#[path = "support/capture.rs"]
mod capture;

#[cfg(feature = "capture")]
use capture::{CaptureExtent, FORMAT, capture_texture, clear_texture, read_texture};

#[cfg(feature = "capture")]
fn main() -> Result<(), Box<dyn Error>> {
    let (device, queue) = request_device()?;
    let mut renderer = WgpuRenderer::new()?;
    let out_dir = std::env::current_dir()?
        .join("target")
        .join("sigil-screenshots");
    std::fs::create_dir_all(&out_dir)?;

    for phase in phases() {
        let path = out_dir.join(format!("phase-R{}-{}.png", phase.number, phase.name));
        match phase.capture {
            PhaseCapture::Single(build) => {
                let (scene, viewport, clear) = build();
                capture_phase(
                    &mut renderer,
                    RenderRequest {
                        device: &device,
                        queue: &queue,
                        scene: &scene,
                        viewport,
                        extent: CaptureExtent::from_viewport(viewport),
                        clear,
                        path: &path,
                    },
                )?;
            }
            PhaseCapture::Split(build) => {
                let scenes = build();
                capture_split_phase(&mut renderer, &device, &queue, &scenes, &path)?;
            }
        }
        let size = path.metadata()?.len();
        println!("{} {} bytes", path.canonicalize()?.display(), size);
    }
    Ok(())
}

#[cfg(not(feature = "capture"))]
fn main() {}

#[cfg(feature = "capture")]
struct Phase {
    number: u8,
    name: &'static str,
    capture: PhaseCapture,
}

#[cfg(feature = "capture")]
type SceneBundle = (CommandScene, (f32, f32), sigil_hal::Color);
#[cfg(feature = "capture")]
type SceneFactory = fn() -> SceneBundle;
#[cfg(feature = "capture")]
type SplitFactory = fn() -> [SceneBundle; 2];

#[derive(Clone, Copy)]
#[cfg(feature = "capture")]
enum PhaseCapture {
    Single(SceneFactory),
    Split(SplitFactory),
}

#[allow(clippy::too_many_lines)] // pure data table — one entry per phase, no logic
#[cfg(feature = "capture")]
fn phases() -> [Phase; 13] {
    [
        Phase {
            number: 1,
            name: "gradient",
            capture: PhaseCapture::Single(scenes::gradient),
        },
        Phase {
            number: 2,
            name: "stroke",
            capture: PhaseCapture::Single(scenes::stroke),
        },
        Phase {
            number: 3,
            name: "corner_radius",
            capture: PhaseCapture::Single(scenes::corner_radius),
        },
        Phase {
            number: 4,
            name: "shadow",
            capture: PhaseCapture::Single(scenes::shadow),
        },
        Phase {
            number: 5,
            name: "inner_shadow",
            capture: PhaseCapture::Single(scenes::inner_shadow),
        },
        Phase {
            number: 6,
            name: "pattern",
            capture: PhaseCapture::Single(scenes::pattern),
        },
        Phase {
            number: 7,
            name: "sprite",
            capture: PhaseCapture::Single(scenes::sprite),
        },
        Phase {
            number: 8,
            name: "icon",
            capture: PhaseCapture::Single(scenes::icon),
        },
        Phase {
            number: 9,
            name: "decoration",
            capture: PhaseCapture::Single(scenes::decoration),
        },
        Phase {
            number: 10,
            name: "glow",
            capture: PhaseCapture::Single(scenes::glow),
        },
        Phase {
            number: 11,
            name: "layer",
            capture: PhaseCapture::Single(scenes::layer_pool),
        },
        Phase {
            number: 12,
            name: "blur",
            capture: PhaseCapture::Single(scenes::backdrop_blur),
        },
        Phase {
            number: 13,
            name: "split",
            capture: PhaseCapture::Split(scenes::screen_split),
        },
    ]
}

#[cfg(feature = "capture")]
fn request_device() -> Result<(wgpu::Device, wgpu::Queue), Box<dyn Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|_| io::Error::other("no GPU adapter available for offscreen capture"))?;

    let device_descriptor = wgpu::DeviceDescriptor {
        label: Some("sigil-renderer capture device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    };
    Ok(pollster::block_on(
        adapter.request_device(&device_descriptor),
    )?)
}

#[derive(Clone, Copy)]
#[cfg(feature = "capture")]
struct RenderRequest<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    scene: &'a CommandScene,
    viewport: (f32, f32),
    extent: CaptureExtent,
    clear: sigil_hal::Color,
    path: &'a Path,
}

#[cfg(feature = "capture")]
fn capture_phase(
    renderer: &mut WgpuRenderer,
    request: RenderRequest<'_>,
) -> Result<(), Box<dyn Error>> {
    let texture = capture_texture(request.device, request.extent);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Keep scene coordinates in logical viewport space while rendering into a
    // larger physical texture. The extra pixels improve screenshot resolution
    // without changing layout, radii, shadow offsets, or stroke widths.
    renderer.submit_screen(
        &ScreenSubmission {
            device: request.device,
            queue: request.queue,
            view: &view,
            format: FORMAT,
            viewport: request.viewport,
            // The capture texture is supersampled (`CaptureExtent::from_viewport`
            // scales the logical viewport up for screenshot resolution); the honest
            // DPI factor is that physical/logical ratio, so screenshot text is sharp.
            scale: request.extent.width as f32 / request.viewport.0.max(1.0),
            clear: Some(request.clear),
        },
        request.scene,
    )?;
    let pixels = read_texture(request.device, request.queue, &texture, request.extent)?;
    image::save_buffer_with_format(
        request.path,
        &pixels,
        request.extent.width,
        request.extent.height,
        ColorType::Rgba8,
        ImageFormat::Png,
    )?;
    Ok(())
}

#[cfg(feature = "capture")]
fn capture_split_phase(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scenes: &[SceneBundle; 2],
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let extent = CaptureExtent {
        width: SPLIT_WIDTH,
        height: SPLIT_HALF_HEIGHT,
    };
    let top = render_split_scene(renderer, device, queue, &scenes[0], extent, true)?;
    let bottom = render_split_scene(renderer, device, queue, &scenes[1], extent, false)?;
    let mut pixels = Vec::with_capacity(top.len() + bottom.len());
    pixels.extend_from_slice(&top);
    pixels.extend_from_slice(&bottom);
    image::save_buffer_with_format(
        path,
        &pixels,
        SPLIT_WIDTH,
        SPLIT_HEIGHT,
        ColorType::Rgba8,
        ImageFormat::Png,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "capture")]
fn render_split_scene(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &SceneBundle,
    extent: CaptureExtent,
    clear_in_submit: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let texture = capture_texture(device, extent);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    if !clear_in_submit {
        clear_texture(device, queue, &view, scene.2);
    }
    renderer.submit_screen(
        &ScreenSubmission {
            device,
            queue,
            view: &view,
            format: FORMAT,
            viewport: SPLIT_VIEWPORT,
            // Split capture extent equals SPLIT_VIEWPORT — logical == physical.
            scale: 1.0,
            clear: clear_in_submit.then_some(scene.2),
        },
        &scene.0,
    )?;
    read_texture(device, queue, &texture, extent)
}
