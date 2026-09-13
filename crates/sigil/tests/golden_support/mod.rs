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

use pollster::FutureExt;
use sigil::hal::{
    Color, ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use sigil::prelude::*;
use sigil::renderer::{CommandScene, OffscreenTarget, WgpuRenderer, emit_scene};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

pub const SETTLE_SAFETY_CAP: u32 = 600;

#[derive(Clone, Copy, Debug)]
pub enum InteractionMode {
    Default,
    Hover,
    Pressed,
    Clicked,
    Focused,
    Disabled,
    HoverAfterFrames { frames: u32, dt: f32 },
    PressedAfterFrames { frames: u32, dt: f32 },
    FocusedAfterFrames { frames: u32, dt: f32 },
}

impl InteractionMode {
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Hover | Self::HoverAfterFrames { .. } => "hover",
            Self::Pressed | Self::PressedAfterFrames { .. } => "pressed",
            Self::Clicked => "clicked",
            Self::Focused | Self::FocusedAfterFrames { .. } => "focused",
            Self::Disabled => "disabled",
        }
    }
}

pub const FILMSTRIP_GAP_PX: u32 = 4;

#[must_use]
pub fn capture_filmstrip(
    build: impl Fn(&mut Cx) -> NodeId,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    frame_count: u32,
    dt: f32,
) -> Option<image::RgbaImage> {
    assert!(
        frame_count >= 1,
        "capture_filmstrip: frame_count must be at least 1",
    );
    let mut frames: Vec<image::RgbaImage> = Vec::with_capacity(frame_count as usize);
    for index in 0..frame_count {
        let mode = if index == 0 {
            InteractionMode::Default
        } else {
            InteractionMode::HoverAfterFrames {
                frames: index - 1,
                dt,
            }
        };
        let frame = render_widget_to_png(&build, mode, size_px, hit_point)?;
        frames.push(frame);
    }
    Some(stitch_horizontal(&frames, FILMSTRIP_GAP_PX))
}

fn stitch_horizontal(frames: &[image::RgbaImage], gap_px: u32) -> image::RgbaImage {
    assert!(!frames.is_empty(), "stitch_horizontal: no frames");
    let frame_w = frames[0].width();
    let height = frames[0].height();
    for frame in frames {
        assert_eq!(frame.width(), frame_w, "filmstrip frame width mismatch");
        assert_eq!(frame.height(), height, "filmstrip frame height mismatch");
    }
    let n = frames.len() as u32;
    let total_w = n * frame_w + n.saturating_sub(1) * gap_px;
    let mut out = image::RgbaImage::from_pixel(total_w, height, image::Rgba([0, 0, 0, 255]));
    for (index, frame) in frames.iter().enumerate() {
        let x_offset = index as u32 * (frame_w + gap_px);
        for y in 0..height {
            for x in 0..frame_w {
                let pixel = *frame.get_pixel(x, y);
                out.put_pixel(x_offset + x, y, pixel);
            }
        }
    }
    out
}

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

#[must_use]
pub fn regen_enabled() -> bool {
    matches!(
        std::env::var("SIGIL_REGEN_GOLDENS").ok().as_deref(),
        Some("1" | "true" | "yes" | "on"),
    )
}

#[must_use]
pub fn real_measure_enabled() -> bool {
    matches!(
        std::env::var("SIGIL_REAL_MEASURE").ok().as_deref(),
        Some("1" | "true" | "yes" | "on"),
    )
}

fn default_text_measure() -> Box<dyn sigil::layout::TextMeasure> {
    if real_measure_enabled() {
        Box::new(sigil::app::DefaultTextMeasure::new())
    } else {
        Box::new(StubMeasure)
    }
}

#[must_use]
pub fn golden_path(stem: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("goldens");
    p.push(format!("{stem}.png"));
    p
}

#[must_use]
pub fn render_widget_to_png(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Option<image::RgbaImage> {
    render_widget_to_png_with(build, mode, size_px, hit_point, default_text_measure())
}

#[must_use]
pub fn render_widget_to_png_with(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil::layout::TextMeasure>,
) -> Option<image::RgbaImage> {
    render_core(build, mode, size_px, hit_point, text_measure, |_, root| {
        root
    })
}

#[must_use]
pub fn render_widget_to_png_scaled(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size_px: [u32; 2],
    scale: f32,
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil::layout::TextMeasure>,
) -> Option<image::RgbaImage> {
    let logical = [logical_size_px[0] as f32, logical_size_px[1] as f32];
    let physical = [
        (logical[0] * scale).round() as u32,
        (logical[1] * scale).round() as u32,
    ];
    render_core_scaled(
        build,
        mode,
        logical,
        physical,
        hit_point,
        text_measure,
        |_, root| root,
    )
}

#[must_use]
pub fn render_widget_to_png_focus_descendant(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Option<image::RgbaImage> {
    render_core(
        build,
        mode,
        size_px,
        hit_point,
        Box::new(StubMeasure),
        |cx, root| find_first_focusable(cx, root).unwrap_or(root),
    )
}

fn render_core(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil::layout::TextMeasure>,
    focus_target: impl Fn(&Cx, NodeId) -> NodeId,
) -> Option<image::RgbaImage> {
    let logical = [size_px[0] as f32, size_px[1] as f32];
    render_core_scaled(
        build,
        mode,
        logical,
        size_px,
        hit_point,
        text_measure,
        focus_target,
    )
}

fn render_core_scaled(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size: [f32; 2],
    physical_size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil::layout::TextMeasure>,
    focus_target: impl Fn(&Cx, NodeId) -> NodeId,
) -> Option<image::RgbaImage> {
    let mut image: Option<image::RgbaImage> = None;
    sigil::reactive::Runtime::scope("sigil-visual-golden", || {
        let mut cx = Cx::new_root();
        let root = build(&mut cx);
        let target = focus_target(&cx, root);

        let [viewport_w, viewport_h] = logical_size;
        let mut layout = LayoutEngine::with_text_measure(text_measure);
        layout
            .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
            .expect("layout");

        let mut router = EventRouter::new();
        drive_state(&mut router, &mut cx, &layout, target, mode, hit_point);

        layout
            .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
            .expect("layout-after-state");

        let scene = emit_scene(&cx, root);
        let Some((device, queue)) = request_device() else {
            return;
        };
        image = Some(render_scene_to_image(
            &device,
            &queue,
            &scene,
            (viewport_w, viewport_h),
            physical_size_px,
        ));
    });
    image
}

fn find_first_focusable(cx: &Cx, id: NodeId) -> Option<NodeId> {
    let node = cx.get(id)?;
    if node.a11y.focusable {
        return Some(id);
    }
    node.children()
        .iter()
        .find_map(|child| find_first_focusable(cx, *child))
}

pub fn assert_png_matches_golden(actual: &image::RgbaImage, golden: &Path, max_delta_pct: f32) {
    const CHANNEL_TOLERANCE: u8 = 2;

    if regen_enabled() {
        guard_regen_does_not_erase_content(actual, golden);
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

fn ink(image: &image::RgbaImage) -> usize {
    use std::collections::HashMap;
    let mut census: HashMap<[u8; 4], usize> = HashMap::new();
    for px in image.pixels() {
        *census.entry(px.0).or_default() += 1;
    }
    let background = census.values().copied().max().unwrap_or(0);
    (image.width() as usize * image.height() as usize).saturating_sub(background)
}

fn guard_regen_does_not_erase_content(actual: &image::RgbaImage, golden: &Path) {
    const RETAINED_FRACTION: f32 = 0.5;

    if std::env::var("SIGIL_REGEN_ALLOW_CONTENT_LOSS").as_deref() == Ok("1") {
        return;
    }
    let Ok(previous) = image::open(golden) else {
        return;
    };
    let before = ink(&previous.to_rgba8());
    let after = ink(actual);
    #[allow(clippy::cast_precision_loss)]
    let retained = if before == 0 {
        1.0
    } else {
        after as f32 / before as f32
    };
    assert!(
        retained >= RETAINED_FRACTION,
        "re-bake of {} would erase {:.0}% of the drawn content \
         ({before} -> {after} non-background pixels).\n\
         A re-bake overwrites the reference, so a regression that deletes \
         content would certify itself. Look at the render before accepting \
         it; if the reduction really is intended, set \
         SIGIL_REGEN_ALLOW_CONTENT_LOSS=1.",
        golden.display(),
        (1.0 - retained) * 100.0,
    );
}

pub fn assert_goldens_differ(name_a: &str, name_b: &str) {
    let path_a = golden_path(name_a);
    let path_b = golden_path(name_b);
    let bytes_a = std::fs::read(&path_a)
        .unwrap_or_else(|err| panic!("read golden {}: {err}", path_a.display()));
    let bytes_b = std::fs::read(&path_b)
        .unwrap_or_else(|err| panic!("read golden {}: {err}", path_b.display()));
    assert_ne!(
        bytes_a, bytes_b,
        "goldens '{name_a}' and '{name_b}' are byte-identical — the state \
         change they exist to capture renders no visible difference. Fix the \
         widget, then re-bake with SIGIL_REGEN_GOLDENS=1.",
    );
}

fn drive_state(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    focus_target: NodeId,
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
        InteractionMode::Clicked => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
            router.route(cx, layout, pointer_up(hit.0, hit.1));
        }
        InteractionMode::Focused => {
            router.set_focus(cx, Some(focus_target));
        }
        InteractionMode::HoverAfterFrames { frames, dt } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            tick_animation_runtime(frames, dt);
        }
        InteractionMode::PressedAfterFrames { frames, dt } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
            tick_animation_runtime(frames, dt);
        }
        InteractionMode::FocusedAfterFrames { frames, dt } => {
            router.set_focus(cx, Some(focus_target));
            tick_animation_runtime(frames, dt);
        }
    }
}

fn tick_animation_runtime(frames: u32, dt: f32) {
    if frames == u32::MAX {
        for _ in 0..SETTLE_SAFETY_CAP {
            if sigil::animation_runtime::active_count() == 0 {
                return;
            }
            sigil::animation_runtime::tick(dt);
        }
        let remaining = sigil::animation_runtime::active_count();
        assert!(
            remaining == 0,
            "golden harness: animation runtime did not settle within \
             SETTLE_SAFETY_CAP ({SETTLE_SAFETY_CAP}) frames at dt={dt}. \
             Active animators remaining: {remaining}. A widget under test is \
             likely retargeting every frame.",
        );
        return;
    }
    for _ in 0..frames {
        sigil::animation_runtime::tick(dt);
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

fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Up,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

fn request_device() -> Option<(wgpu::Device, wgpu::Queue)> {
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
