//! Headless PNG capture for renderer phase showcase scenes.

#![allow(clippy::indexing_slicing)] // Examples may slice staging buffers; see docs/plans/native-ui/hygiene-cleanup-framework.md.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::print_stdout)]
#![allow(missing_docs)]

#[cfg(windows)]
use windows_core as _;

use std::error::Error;
use std::io;
use std::path::Path;

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image::{ColorType, ImageFormat};
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_renderer::{CommandScene, ScreenSubmission, WgpuRenderer, scenes};
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;

const SPLIT_WIDTH: u32 = 1280;
const SPLIT_HALF_HEIGHT: u32 = 400;
const SPLIT_HEIGHT: u32 = SPLIT_HALF_HEIGHT * 2;
const SPLIT_VIEWPORT: (f32, f32) = (SPLIT_WIDTH as f32, SPLIT_HALF_HEIGHT as f32);

#[path = "support/capture.rs"]
mod capture;

use capture::{CaptureExtent, FORMAT, capture_texture, clear_texture, read_texture};

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

struct Phase {
    number: u8,
    name: &'static str,
    capture: PhaseCapture,
}

type SceneBundle = (CommandScene, (f32, f32), sigil_hal::Color);
type SceneFactory = fn() -> SceneBundle;
type SplitFactory = fn() -> [SceneBundle; 2];

#[derive(Clone, Copy)]
enum PhaseCapture {
    Single(SceneFactory),
    Split(SplitFactory),
}

#[allow(clippy::too_many_lines)] // pure data table — one entry per phase, no logic
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
struct RenderRequest<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    scene: &'a CommandScene,
    viewport: (f32, f32),
    extent: CaptureExtent,
    clear: sigil_hal::Color,
    path: &'a Path,
}

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
            clear: clear_in_submit.then_some(scene.2),
        },
        &scene.0,
    )?;
    read_texture(device, queue, &texture, extent)
}
