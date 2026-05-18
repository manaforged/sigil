//! Visual-golden PNG harness for sigil widgets.
//!
//! Public API mirrors the brief: drive a widget into one of five interaction
//! states, render through the wgpu offscreen path, then assert against a
//! committed PNG byte-for-byte (with a small pixel-delta budget). Setting
//! `SIGIL_REGEN_GOLDENS=1` rewrites the file instead of asserting.
//!
//! `InteractionState` is intentionally NOT an external lever in sigil — each
//! widget owns its own [`crate::interaction::InteractionState`] reactive
//! bundle, driven by `EventRouter` pointer/key events. So the harness instead
//! takes an [`InteractionMode`] enum and applies it via the same event-router
//! pathway the existing `widget_states.rs` integration tests use.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::print_stdout, clippy::doc_markdown)]
#![allow(clippy::too_many_lines, clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments, clippy::missing_const_for_fn)]
#![allow(unreachable_pub, dead_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use pollster::FutureExt as _;

use sigil::prelude::*;
use sigil_hal::{
    Color, ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use sigil_renderer::{CommandScene, OffscreenTarget, WgpuRenderer, emit_scene};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

/// Which interaction state to drive the widget into before capture.
#[derive(Clone, Copy, Debug)]
pub enum InteractionMode {
    /// Resting / neutral.
    Default,
    /// Pointer is over the widget but no button is down.
    Hover,
    /// Pointer is down inside the widget (active press).
    Pressed,
    /// Keyboard focus on the widget.
    Focused,
    /// Widget disabled (the widget builder must own this — harness skips events).
    Disabled,
}

impl InteractionMode {
    /// Lowercase tag used in golden filenames.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Hover => "hover",
            Self::Pressed => "pressed",
            Self::Focused => "focused",
            Self::Disabled => "disabled",
        }
    }
}

/// Minimal text measurement stub matching the existing widget tests.
struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

/// Return `true` when `SIGIL_REGEN_GOLDENS=1` is set in the environment.
#[must_use]
pub fn regen_enabled() -> bool {
    matches!(
        std::env::var("SIGIL_REGEN_GOLDENS").ok().as_deref(),
        Some("1" | "true" | "yes" | "on"),
    )
}

/// Resolve a golden PNG path relative to this crate.
#[must_use]
pub fn golden_path(stem: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("goldens");
    p.push(format!("{stem}.png"));
    p
}

/// Full render → image::RgbaImage. `build` constructs the widget; `mode` drives
/// the state into one of the five interaction modes through `EventRouter`.
///
/// `hit_point` is the logical-pixel point used to drive Hover / Pressed events.
/// Choose it so the pointer lands inside the widget's hit-test surface.
///
/// Skips with `None` if no wgpu adapter is available on the host.
#[must_use]
pub fn render_widget_to_png(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Option<image::RgbaImage> {
    let mut image: Option<image::RgbaImage> = None;
    sigil::reactive::Runtime::scope("sigil-visual-golden", || {
        let mut cx = Cx::new_root();
        let root = build(&mut cx);

        let viewport_w = size_px[0] as f32;
        let viewport_h = size_px[1] as f32;
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
            .expect("layout");

        let mut router = EventRouter::new();
        drive_state(&mut router, &mut cx, &layout, root, mode, hit_point);

        // Re-run layout: state changes may have updated reactive size signals.
        layout
            .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
            .expect("layout-after-state");

        let scene = emit_scene(&cx, root);
        let Some((device, queue)) = try_request_device() else {
            return;
        };
        image = Some(render_scene_to_image(
            &device,
            &queue,
            &scene,
            (viewport_w, viewport_h),
            size_px,
        ));
    });
    image
}

/// Either assert the captured image matches `golden_path`, or — when
/// `SIGIL_REGEN_GOLDENS=1` — overwrite the golden file with `actual`.
///
/// `max_delta_pct` is the maximum percentage of differing pixels permitted
/// (range 0.0..=100.0). A pixel is "different" when any RGBA channel differs
/// by more than `CHANNEL_TOLERANCE`.
pub fn assert_png_matches_golden(actual: &image::RgbaImage, golden: &Path, max_delta_pct: f32) {
    const CHANNEL_TOLERANCE: u8 = 2;

    if regen_enabled() {
        std::fs::create_dir_all(golden.parent().expect("golden parent")).expect("mkdir goldens");
        actual.save(golden).expect("write golden png");
        println!("REGEN: wrote {}", golden.display());
        return;
    }

    assert!(
        golden.exists(),
        "golden file missing: {}\n\
         Run with SIGIL_REGEN_GOLDENS=1 to create initial bakes.",
        golden.display(),
    );

    let expected = image::open(golden)
        .unwrap_or_else(|err| panic!("decode golden {}: {err}", golden.display()))
        .to_rgba8();

    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "golden dimension mismatch for {}",
        golden.display(),
    );

    let total = u64::from(actual.width()) * u64::from(actual.height());
    let mut differing: u64 = 0;
    for (a, b) in actual.pixels().zip(expected.pixels()) {
        let diff =
            a.0.iter()
                .zip(b.0.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE);
        if diff {
            differing += 1;
        }
    }
    let pct = (differing as f64 / total as f64) * 100.0;
    assert!(
        pct as f32 <= max_delta_pct,
        "golden {} differs in {:.3}% of pixels (>{} channel delta), \
         budget {:.3}%. Re-bake with SIGIL_REGEN_GOLDENS=1 if intentional.",
        golden.display(),
        pct,
        CHANNEL_TOLERANCE,
        max_delta_pct,
    );
}

// ---- state driving -----------------------------------------------------------

fn drive_state(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    root: NodeId,
    mode: InteractionMode,
    hit: (f32, f32),
) {
    match mode {
        InteractionMode::Default | InteractionMode::Disabled => {}
        InteractionMode::Hover => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
        }
        InteractionMode::Pressed => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
        }
        InteractionMode::Focused => {
            router.set_focus(cx, Some(root));
        }
    }
}

fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

// ---- wgpu offscreen plumbing -------------------------------------------------

fn try_request_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .block_on()
        .ok()?;
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("visual-golden device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()
}

fn render_scene_to_image(
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

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport,
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

    let mapped = slice.get_mapped_range();
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
