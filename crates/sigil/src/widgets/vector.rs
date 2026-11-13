use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::scene::SceneBuilder;
use crate::scene::canvas::CanvasPaint;
use crate::scene::command::{
    DrawPath, FillRule, PathPaint, PathStroke, StrokeCap, StrokeJoin, Transform,
};
use crate::scene::geometry::IDENTITY;
use crate::style::{Color, ColorValue, Theme};
use crate::widgets::Widget;
use crate::widgets::layout_ext::LayoutOverrides;

mod key;
mod paint;
mod values;

pub(crate) use crate::scene::command::PathCmd;
pub(crate) use values::PathFill;

type Recorded = RefCell<Option<(Rect, u64, DrawPath)>>;

/// Builder of a vector path.
#[derive(Debug)]
#[must_use]
pub struct PathBuilder {
    width: u32,
    height: u32,
    cmds: Vec<PathCmd>,
    fill: Option<PathFill>,
    stroke: Option<(ColorValue, f32)>,
    rule: FillRule,
    cap: Option<StrokeCap>,
    join: Option<StrokeJoin>,
    dash: Vec<f32>,
    fit: Option<LayoutStyle>,
    layout_overrides: LayoutOverrides,
    non_scaling: bool,
}

/// A vector path.
pub const fn path(width: u32, height: u32) -> PathBuilder {
    PathBuilder {
        width,
        height,
        cmds: Vec::new(),
        fill: None,
        stroke: None,
        rule: FillRule::NonZero,
        cap: None,
        join: None,
        dash: Vec::new(),
        fit: None,
        layout_overrides: LayoutOverrides::new(),
        non_scaling: false,
    }
}

impl PathBuilder {
    /// Starts a new subpath at this point.
    pub fn move_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::MoveTo(x, y));
        self
    }

    /// Adds a straight line to this point.
    pub fn line_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::LineTo(x, y));
        self
    }

    /// Adds a quadratic curve to this point.
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::QuadTo(cx, cy, x, y));
        self
    }

    /// Adds a cubic curve to this point.
    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y));
        self
    }

    /// Closes the current subpath.
    pub fn close(mut self) -> Self {
        self.cmds.push(PathCmd::Close);
        self
    }

    /// Sets a solid fill.
    pub fn fill(mut self, color: impl Into<ColorValue>) -> Self {
        self.fill = Some(PathFill::Solid(color.into()));
        self
    }

    /// Sets a linear gradient fill.
    pub fn fill_linear(
        mut self,
        start: (f32, f32),
        end: (f32, f32),
        stops: Vec<(f32, Color)>,
    ) -> Self {
        self.fill = Some(PathFill::Linear { start, end, stops });
        self
    }

    /// Sets a radial gradient fill.
    pub fn fill_radial(
        mut self,
        center: (f32, f32),
        radius: f32,
        stops: Vec<(f32, Color)>,
    ) -> Self {
        self.fill = Some(PathFill::Radial {
            center,
            radius,
            stops,
        });
        self
    }

    /// Fills with the even-odd rule instead of nonzero.
    pub const fn even_odd(mut self) -> Self {
        self.rule = FillRule::EvenOdd;
        self
    }

    /// Strokes the path in `color` with a line of `width`.
    pub fn stroke(mut self, color: impl Into<ColorValue>, width: f32) -> Self {
        self.stroke = Some((color.into(), width));
        self
    }

    /// Sets the shape of the stroke's open ends.
    pub const fn cap(mut self, cap: StrokeCap) -> Self {
        self.cap = Some(cap);
        self
    }

    /// Sets the shape where stroke segments meet.
    pub const fn join(mut self, join: StrokeJoin) -> Self {
        self.join = Some(join);
        self
    }

    /// Dashes the stroke with alternating on and off lengths.
    pub fn dash(mut self, lengths: &[f32]) -> Self {
        self.dash = lengths.to_vec();
        self
    }

    /// Keeps the stroke width when the path scales.
    pub const fn non_scaling_stroke(mut self) -> Self {
        self.non_scaling = true;
        self
    }

    /// Scales the path to fill its box.
    pub fn fit(self) -> Self {
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let (width, height) = (self.width as f32, self.height as f32);
        self.fit_box(width, height)
    }

    /// Scales the path from this view size to its box.
    pub fn fit_size(self, logical: f32) -> Self {
        self.fit_box(logical, logical)
    }

    fn fit_box(mut self, width: f32, height: f32) -> Self {
        self.fit = Some(fixed(width, height).min_size(width, height));
        self
    }

    /// Record a vector path into a `canvas` scene.
    pub fn draw(self, scene: &mut SceneBuilder, destination: Rect, theme: &Theme) {
        if let Some(draw) = self.build_path(destination, theme) {
            scene.draw_path(draw);
        }
    }

    /// Resolves and builds a [`DrawPath`] command for `destination` using `theme`.
    #[must_use]
    pub fn build_path(&self, destination: Rect, theme: &Theme) -> Option<DrawPath> {
        if self.cmds.is_empty() || (self.fill.is_none() && self.stroke.is_none()) {
            return None;
        }
        let (fill, stroke) = paint::resolve(self.fill.as_ref(), self.stroke, theme);
        let key = paint::path_key(self, fill.as_ref(), stroke);
        let view = self.view(destination);
        let (outline, transform, baked) = if self.non_scaling {
            (paint::bake(&self.cmds, view), IDENTITY, Some(view))
        } else {
            (Arc::from(self.cmds.as_slice()), view, None)
        };
        let mut draw = DrawPath::new(outline, transform, key);
        if let Some(fill) = fill {
            let fill = baked.map_or_else(|| fill.clone(), |frame| paint::map(&fill, frame));
            draw = draw.with_fill(self.rule, fill);
        }
        if let Some((color, width)) = stroke {
            draw = draw.with_stroke(self.path_stroke(color, width));
        }
        Some(draw)
    }

    fn path_stroke(&self, color: Color, width: f32) -> PathStroke {
        let (cap, join) = if self.non_scaling {
            (StrokeCap::Round, StrokeJoin::Round)
        } else {
            (StrokeCap::Butt, StrokeJoin::Miter)
        };
        PathStroke {
            paint: PathPaint::Solid(color),
            width,
            cap: self.cap.unwrap_or(cap),
            join: self.join.unwrap_or(join),
            dash: self.dash.iter().copied().collect(),
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "path extents are small positive integers"
    )]
    fn view(&self, destination: Rect) -> Transform {
        let sx = destination.width / self.width.max(1) as f32;
        let sy = destination.height / self.height.max(1) as f32;
        [
            [sx, 0.0, destination.origin.x],
            [0.0, sy, destination.origin.y],
        ]
    }

    fn record(&self, last: &Recorded, scene: &mut SceneBuilder, rect: Rect, theme: &Theme) {
        let stamp = paint::theme_key(self, theme);
        let mut last = last.borrow_mut();
        let fresh = last
            .as_ref()
            .is_none_or(|(at, seen, _)| *at != rect || *seen != stamp);
        if fresh {
            *last = self.build_path(rect, theme).map(|draw| (rect, stamp, draw));
        }
        if let Some((_, _, draw)) = last.as_ref() {
            scene.draw_path(draw.clone());
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "path extents are small positive integers"
    )]
    fn root(mut self) -> NodeBuilder {
        let layout = self
            .fit
            .take()
            .unwrap_or_else(|| fixed(self.width as f32, self.height as f32));
        let path = Rc::new(self);
        let last = Recorded::default();
        let paint = CanvasPaint::themed(Rc::new(move |scene, rect, theme| {
            path.record(&last, scene, rect, theme);
        }));
        container().layout(layout).mutate(move |node| {
            let _ = node.attach(paint);
        })
    }
}

fn fixed(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::block().fixed_size(width, height).shrink(0.0)
}

builder_impls!([] PathBuilder: widget, node(pub(crate)));

#[cfg(test)]
#[path = "vector_tests.rs"]
mod tests;
