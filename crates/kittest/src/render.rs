use std::rc::Rc;

use sigil::geometry::{Point, Rect, Size};
use sigil::kernel::{Cx, NodeId};
use sigil::layout::{LayoutEngine, TextMeasure};
use sigil::prelude::*;
use sigil::scene::emit_scene;

use crate::gpu::{self, render_scene_to_image};
use crate::interaction::{self, drive_state};
use crate::{
    Error, ErrorKind, Harness, InteractionMode, Rendered, bounds, real_text_measure,
    stub_text_measure,
};

const FILMSTRIP_GAP_PX: u32 = 4;

type MeasureFn = Rc<dyn Fn() -> Box<dyn TextMeasure>>;
type Capture = (image::RgbaImage, Option<Rect>);

impl Harness {
    /// Starts an offscreen render of one widget.
    pub fn render<B: IntoNode>(build: B) -> Render<B> {
        Render {
            build,
            mode: InteractionMode::Default,
            size: [200, 60],
            scale: None,
            hit: (0.0, 0.0),
            measure: Rc::new(real_text_measure),
            focus_first: false,
        }
    }
}

/// An offscreen render of one widget in an interaction state.
#[must_use]
pub struct Render<B> {
    build: B,
    mode: InteractionMode,
    size: [u32; 2],
    scale: Option<f32>,
    hit: (f32, f32),
    measure: MeasureFn,
    focus_first: bool,
}

impl<B> std::fmt::Debug for Render<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Render")
            .field("mode", &self.mode)
            .field("size", &self.size)
            .field("scale", &self.scale)
            .field("hit", &self.hit)
            .field("focus_first", &self.focus_first)
            .finish_non_exhaustive()
    }
}

impl<B> Render<B> {
    /// Sets the interaction state.
    pub const fn mode(mut self, mode: InteractionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the logical size in pixels.
    pub const fn size(mut self, size: [u32; 2]) -> Self {
        self.size = size;
        self
    }

    /// Sets the device scale; the image is the logical size times this.
    pub const fn scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Sets the pointer position of the interaction state.
    pub const fn hit(mut self, hit: (f32, f32)) -> Self {
        self.hit = hit;
        self
    }

    /// Sets the text measure, called once per render.
    pub fn measure(mut self, measure: impl Fn() -> Box<dyn TextMeasure> + 'static) -> Self {
        self.measure = Rc::new(measure);
        self
    }

    /// Measures each text node as a fixed 40x16 box.
    pub fn stub(self) -> Self {
        self.measure(stub_text_measure)
    }

    /// Drives the first focusable descendant instead of the root.
    pub const fn focus_first(mut self) -> Self {
        self.focus_first = true;
        self
    }

    fn target(&self) -> Extent {
        let logical = self.size.map(crate::extent_f32);
        let physical = self.scale.map_or(self.size, |scale| {
            self.size.map(|extent| physical_extent(extent, scale))
        });
        Extent { logical, physical }
    }
}

impl<B: IntoNode> Render<B> {
    /// Renders the image, or `None` without an adapter.
    pub fn image(self) -> Rendered {
        self.capture(&mut || false)
            .map(|captured| captured.map(|(image, _)| image))
    }

    /// Renders the image and the top layer bounds; `settle` runs layout again while it returns true.
    pub fn capture(self, settle: &mut dyn FnMut() -> bool) -> Result<Option<Capture>, Error> {
        let target = self.target();
        let Self {
            build,
            mode,
            hit,
            measure,
            focus_first,
            ..
        } = self;
        let drive = Drive {
            mode,
            hit,
            focus_first,
        };
        capture(build, target, measure(), drive, settle)
    }
}

impl<F, N> Render<F>
where
    F: Fn() -> N,
    N: IntoNode,
{
    /// Renders `frames` hover frames `dt` seconds apart, side by side.
    pub fn filmstrip(self, frames: u32, dt: f32) -> Rendered {
        let target = self.target();
        let mut images = Vec::new();
        for index in 0..frames.max(1) {
            let mode = if index == 0 {
                InteractionMode::Default
            } else {
                InteractionMode::HoverAfterFrames {
                    frames: index - 1,
                    dt,
                }
            };
            let drive = Drive {
                mode,
                hit: self.hit,
                focus_first: self.focus_first,
            };
            let measure = (self.measure)();
            let captured = capture(&self.build, target, measure, drive, &mut || false)?;
            let Some((image, _)) = captured else {
                return Ok(None);
            };
            images.push(image);
        }
        Ok(Some(stitch_horizontal(&images, target.physical)))
    }
}

fn stitch_horizontal(frames: &[image::RgbaImage], size_px: [u32; 2]) -> image::RgbaImage {
    let [frame_w, height] = size_px;
    let count = u32::try_from(frames.len()).unwrap_or(u32::MAX);
    let total_w = count
        .saturating_mul(frame_w)
        .saturating_add(count.saturating_sub(1).saturating_mul(FILMSTRIP_GAP_PX));
    let mut out = image::RgbaImage::from_pixel(total_w, height, image::Rgba([0, 0, 0, 255]));
    let mut x_offset = 0_i64;
    for frame in frames {
        image::imageops::replace(&mut out, frame, x_offset, 0);
        x_offset += i64::from(frame_w) + i64::from(FILMSTRIP_GAP_PX);
    }
    out
}

#[derive(Clone, Copy)]
struct Extent {
    logical: [f32; 2],
    physical: [u32; 2],
}

#[derive(Clone, Copy)]
struct Drive {
    mode: InteractionMode,
    hit: (f32, f32),
    focus_first: bool,
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn physical_extent(logical: u32, scale: f32) -> u32 {
    (crate::extent_f32(logical) * scale).round().max(0.0) as u32
}

fn capture(
    build: impl IntoNode,
    target: Extent,
    text_measure: Box<dyn TextMeasure>,
    drive: Drive,
    settle: &mut dyn FnMut() -> bool,
) -> Result<Option<Capture>, Error> {
    sigil::reactive::Owner::scope("sigil-visual-golden", || {
        let [viewport_w, viewport_h] = target.logical;
        sigil::app::viewport_size().set(Size::new(viewport_w, viewport_h));
        let mut cx = Cx::new_root();
        let root = cx.add(build);
        let viewport = Constraint::fixed(viewport_w, viewport_h);
        let mut layout = LayoutEngine::new(text_measure);
        let relayout = |tree: &mut Cx, engine: &mut LayoutEngine| {
            engine
                .compute(tree, root, viewport)
                .map_err(|err| Error::caused(ErrorKind::Layout, err))
        };
        relayout(&mut cx, &mut layout)?;
        while settle() {
            relayout(&mut cx, &mut layout)?;
        }
        let focus = if drive.focus_first {
            find_first_focusable(&cx, root).unwrap_or(root)
        } else {
            root
        };
        drive_state(&mut cx, &layout, focus, drive.mode, drive.hit);
        relayout(&mut cx, &mut layout)?;
        interaction::advance(drive.mode, || relayout(&mut cx, &mut layout))?;
        let scene = emit_scene(&cx, root, None);
        let frame = Rect::new(Point::new(0.0, 0.0), viewport_w, viewport_h);
        let top_layer = bounds::top_layer(&cx, &layout, root, frame);
        let Some(mut renderer) = gpu::headless_renderer()? else {
            return Ok(None);
        };
        let image = render_scene_to_image(
            &mut renderer,
            &scene,
            (viewport_w, viewport_h),
            target.physical,
        )?;
        Ok(Some((image, top_layer)))
    })
}

fn find_first_focusable(cx: &Cx, id: NodeId) -> Option<NodeId> {
    let node = cx.get(id)?;
    if node.a11y.focusable && node.a11y.tab_stop.is_none_or(|stop| stop.peek()) {
        return Some(id);
    }
    node.children()
        .iter()
        .find_map(|child| find_first_focusable(cx, *child))
}
