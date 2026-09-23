#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::print_stdout, clippy::doc_markdown)]
#![allow(clippy::too_many_lines, clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments, clippy::missing_const_for_fn)]
#![allow(unreachable_pub, dead_code)]

use sigil::prelude::*;
use sigil::renderer::emit_scene;

mod compare;
pub use compare::{assert_goldens_differ_in, assert_png_matches_golden, golden_path_in};
pub mod gpu;
use gpu::{render_scene_to_image, request_device};
mod interaction;
use interaction::drive_state;

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

#[macro_export]
macro_rules! golden_path {
    ($stem:expr) => {
        $crate::golden_path_in(env!("CARGO_MANIFEST_DIR"), $stem)
    };
}

#[macro_export]
macro_rules! assert_goldens_differ {
    ($name_a:expr, $name_b:expr) => {
        $crate::assert_goldens_differ_in(env!("CARGO_MANIFEST_DIR"), $name_a, $name_b)
    };
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

        interaction::advance(mode, || {
            layout
                .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
                .expect("layout-after-animation");
        });

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
