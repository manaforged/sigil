use sigil::layout::TextMeasure;
use sigil::prelude::*;
use sigil::scene::emit_scene;

mod bounds;
mod compare;
pub use compare::{
    assert_goldens_differ_in, assert_png_matches_golden, assert_rendered_matches_golden,
    assert_text_matches_golden, golden_path_in,
};
mod error;
pub use error::Error;
pub mod gpu;
use gpu::{render_scene_to_image, shared_device};
mod harness;
pub use harness::{Harness, Node, Query, SNAPSHOT_MAX_DELTA_PCT, Target};
pub mod input;
mod interaction;
mod measure;
pub mod parity;
pub mod scene_gen;
mod settle;
use interaction::drive_state;
pub use measure::{CharMeasure, FixedMeasure};

pub const SETTLE_SAFETY_CAP: u32 = 600;

pub type Rendered = Result<Option<image::RgbaImage>, Error>;

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
    Dragged { dx: f32, dy: f32 },
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
            Self::Dragged { .. } => "dragged",
        }
    }
}

pub const FILMSTRIP_GAP_PX: u32 = 4;

#[allow(
    clippy::too_many_arguments,
    reason = "a filmstrip needs the widget, its size, the hit point, the frame count, the step, and the text measure"
)]
pub fn capture_filmstrip(
    build: impl Fn(&mut Cx) -> NodeId,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    frame_count: u32,
    dt: f32,
    text_measure: impl Fn() -> Box<dyn TextMeasure>,
) -> Rendered {
    let mut frames: Vec<image::RgbaImage> = Vec::new();
    for index in 0..frame_count.max(1) {
        let mode = if index == 0 {
            InteractionMode::Default
        } else {
            InteractionMode::HoverAfterFrames {
                frames: index - 1,
                dt,
            }
        };
        let Some(frame) =
            render_widget_to_png_with(&build, mode, size_px, hit_point, text_measure())?
        else {
            return Ok(None);
        };
        frames.push(frame);
    }
    Ok(Some(stitch_horizontal(&frames, size_px, FILMSTRIP_GAP_PX)))
}

fn stitch_horizontal(
    frames: &[image::RgbaImage],
    size_px: [u32; 2],
    gap_px: u32,
) -> image::RgbaImage {
    let [frame_w, height] = size_px;
    let count = u32::try_from(frames.len()).unwrap_or(u32::MAX);
    let total_w = count
        .saturating_mul(frame_w)
        .saturating_add(count.saturating_sub(1).saturating_mul(gap_px));
    let mut out = image::RgbaImage::from_pixel(total_w, height, image::Rgba([0, 0, 0, 255]));
    let mut x_offset = 0_i64;
    for frame in frames {
        image::imageops::replace(&mut out, frame, x_offset, 0);
        x_offset += i64::from(frame_w) + i64::from(gap_px);
    }
    out
}

#[must_use]
pub fn stub_text_measure() -> Box<dyn TextMeasure> {
    Box::new(FixedMeasure {
        width: 40.0,
        height: 16.0,
    })
}

#[must_use]
pub fn real_text_measure() -> Box<dyn TextMeasure> {
    Box::new(sigil::app::DefaultTextMeasure::new())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regen {
    All,
    Adapter,
}

#[must_use]
pub fn regen_mode() -> Option<Regen> {
    match std::env::var("SIGIL_REGEN_GOLDENS").ok().as_deref() {
        Some("1" | "true" | "yes" | "on") => Some(Regen::All),
        Some("adapter") => Some(Regen::Adapter),
        _ => None,
    }
}

#[must_use]
pub fn regen_enabled() -> bool {
    regen_mode().is_some()
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

pub fn render_widget_to_png(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Rendered {
    render_widget_to_png_with(build, mode, size_px, hit_point, real_text_measure())
}

pub fn render_widget_to_png_stub(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Rendered {
    render_widget_to_png_with(build, mode, size_px, hit_point, stub_text_measure())
}

pub fn render_widget_to_png_with(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
) -> Rendered {
    let target = RenderTarget::unscaled(size_px);
    render_core(build, mode, target, hit_point, text_measure, |_, root| root)
}

#[allow(
    clippy::too_many_arguments,
    reason = "a scaled render needs the logical size and the scale beside the render inputs"
)]
pub fn render_widget_to_png_scaled(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size_px: [u32; 2],
    scale: f32,
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
) -> Rendered {
    let target = RenderTarget::scaled(logical_size_px, scale);
    render_core(build, mode, target, hit_point, text_measure, |_, root| root)
}

#[allow(
    clippy::too_many_arguments,
    reason = "a scaled render needs the logical size and the scale beside the render inputs"
)]
pub fn render_widget_to_png_scaled_focus_descendant(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size_px: [u32; 2],
    scale: f32,
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
) -> Rendered {
    let target = RenderTarget::scaled(logical_size_px, scale);
    render_core(
        build,
        mode,
        target,
        hit_point,
        text_measure,
        focus_descendant,
    )
}

pub struct Captured {
    pub image: image::RgbaImage,
    pub top_layer: Option<sigil::hal::Rect>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "a scaled render needs the logical size and the scale beside the render inputs"
)]
pub fn capture_widget_scaled_focus_descendant(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size_px: [u32; 2],
    scale: f32,
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
) -> Result<Option<Captured>, Error> {
    let target = RenderTarget::scaled(logical_size_px, scale);
    capture_core(
        build,
        mode,
        target,
        hit_point,
        text_measure,
        focus_descendant,
    )
}

fn focus_descendant(cx: &Cx, root: NodeId) -> NodeId {
    find_first_focusable(cx, root).unwrap_or(root)
}

pub fn render_widget_to_png_focus_descendant(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
) -> Rendered {
    render_core(
        build,
        mode,
        RenderTarget::unscaled(size_px),
        hit_point,
        text_measure,
        focus_descendant,
    )
}

#[derive(Clone, Copy)]
struct RenderTarget {
    logical: [f32; 2],
    physical: [u32; 2],
}

impl RenderTarget {
    fn unscaled(size_px: [u32; 2]) -> Self {
        Self {
            logical: size_px.map(extent_f32),
            physical: size_px,
        }
    }

    fn scaled(logical_size_px: [u32; 2], scale: f32) -> Self {
        Self {
            logical: logical_size_px.map(extent_f32),
            physical: logical_size_px.map(|extent| physical_extent(extent, scale)),
        }
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) const fn extent_f32(value: u32) -> f32 {
    value as f32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn physical_extent(logical: u32, scale: f32) -> u32 {
    (extent_f32(logical) * scale).round().max(0.0) as u32
}

#[allow(
    clippy::too_many_arguments,
    reason = "every public render entry point funnels its inputs through this function"
)]
fn render_core(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    target: RenderTarget,
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
    focus_target: impl Fn(&Cx, NodeId) -> NodeId,
) -> Rendered {
    capture_core(build, mode, target, hit_point, text_measure, focus_target)
        .map(|captured| captured.map(|captured| captured.image))
}

#[allow(
    clippy::too_many_arguments,
    reason = "every public render entry point funnels its inputs through this function"
)]
fn capture_core(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    target: RenderTarget,
    hit_point: (f32, f32),
    text_measure: Box<dyn TextMeasure>,
    focus_target: impl Fn(&Cx, NodeId) -> NodeId,
) -> Result<Option<Captured>, Error> {
    sigil::reactive::Runtime::scope("sigil-visual-golden", || {
        let mut cx = Cx::new_root();
        let root = build(&mut cx);
        let focus = focus_target(&cx, root);
        let [viewport_w, viewport_h] = target.logical;
        let viewport = Size::fixed(viewport_w, viewport_h);
        let mut layout = LayoutEngine::with_text_measure(text_measure);
        let relayout = |tree: &mut Cx, engine: &mut LayoutEngine| {
            engine
                .compute(tree, root, viewport)
                .map_err(|err| Error::Layout(format!("{err:?}")))
        };
        relayout(&mut cx, &mut layout)?;
        drive_state(&mut cx, &layout, focus, mode, hit_point);
        relayout(&mut cx, &mut layout)?;
        interaction::advance(mode, || relayout(&mut cx, &mut layout))?;
        let scene = emit_scene(&cx, root);
        let frame = sigil::hal::Rect::new(sigil::hal::Point::new(0.0, 0.0), viewport_w, viewport_h);
        let top_layer = bounds::top_layer(&cx, &layout, root, frame);
        let Some((device, queue)) = shared_device()? else {
            return Ok(None);
        };
        let image = render_scene_to_image(
            device,
            queue,
            &scene,
            (viewport_w, viewport_h),
            target.physical,
        )?;
        Ok(Some(Captured { image, top_layer }))
    })
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

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/automation.md")]
pub struct AutomationGuide;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/testing.md")]
pub struct TestingGuide;
