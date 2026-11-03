use std::sync::Arc;

use crate::geometry::Rect;
use crate::kernel::{
    Cx, ImageRaster, ImageSource, IntoNode, NodeId, ObjectFit, ThemedImage, cached_image,
    container, image,
};
use crate::layout::LayoutStyle;
use crate::scene::SceneBuilder;
use crate::scene::command::DrawImage;
use crate::style::{Rgba8, Theme};
use crate::widgets::Widget;
use crate::widgets::layout_ext::LayoutOverrides;
use tiny_skia::{FillRule, Paint, Pixmap, Transform};

mod key;
mod raster;
mod values;

use crate::fnv::OFFSET;
use key::{hash_cmd, mix, mix_fill, mix_stroke};
use raster::{build_ts_path, fill_shader, straight_rgba, stroke_path};
pub(crate) use values::{PathCmd, PathColor, PathFill};

type ResolvedPaint = (Option<PathFill>, Option<(Rgba8, f32)>, u64);

/// Builder of a vector path.
pub struct PathNode {
    width: u32,
    height: u32,
    cmds: Vec<PathCmd>,
    fill: Option<PathFill>,
    stroke: Option<(PathColor, f32)>,
    fit: Option<LayoutStyle>,
    layout_overrides: LayoutOverrides,
    scale: u8,
    non_scaling: bool,
}

type RasterSize = (u32, u32, f32);

/// A vector path.
#[must_use]
pub const fn path(width: u32, height: u32) -> PathNode {
    PathNode {
        width,
        height,
        cmds: Vec::new(),
        fill: None,
        stroke: None,
        fit: None,
        layout_overrides: LayoutOverrides::new(),
        scale: 1,
        non_scaling: false,
    }
}

impl PathNode {
    /// Starts a new subpath at this point.
    #[must_use]
    pub fn move_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::MoveTo(x, y));
        self
    }

    /// Adds a straight line to this point.
    #[must_use]
    pub fn line_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::LineTo(x, y));
        self
    }

    /// Adds a quadratic curve to this point.
    #[must_use]
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::QuadTo(cx, cy, x, y));
        self
    }

    /// Adds a cubic curve to this point.
    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    #[must_use]
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y));
        self
    }

    /// Closes the current subpath.
    #[must_use]
    pub fn close(mut self) -> Self {
        self.cmds.push(PathCmd::Close);
        self
    }

    /// Sets a solid fill.
    #[must_use]
    pub fn fill(mut self, color: impl Into<PathColor>) -> Self {
        self.fill = Some(match color.into() {
            PathColor::Rgba(color) => PathFill::Solid(color),
            PathColor::Token(token) => PathFill::Theme(token),
        });
        self
    }

    /// Sets a linear gradient fill.
    #[must_use]
    pub fn fill_linear(
        mut self,
        start: (f32, f32),
        end: (f32, f32),
        stops: Vec<(f32, Rgba8)>,
    ) -> Self {
        self.fill = Some(PathFill::Linear { start, end, stops });
        self
    }

    /// Sets a radial gradient fill.
    #[must_use]
    pub fn fill_radial(
        mut self,
        center: (f32, f32),
        radius: f32,
        stops: Vec<(f32, Rgba8)>,
    ) -> Self {
        self.fill = Some(PathFill::Radial {
            center,
            radius,
            stops,
        });
        self
    }

    /// Sets the stroke.
    #[must_use]
    pub fn stroke(mut self, color: impl Into<PathColor>, width: f32) -> Self {
        self.stroke = Some((color.into(), width));
        self
    }

    /// Keeps the stroke width when the path scales.
    #[must_use]
    pub const fn non_scaling_stroke(mut self) -> Self {
        self.non_scaling = true;
        self
    }

    /// Scales the path to fill its box.
    #[must_use]
    pub fn fit(self) -> Self {
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let (width, height) = (self.width as f32, self.height as f32);
        self.fit_box(width, height)
    }

    /// Scales the path from this view size to its box.
    #[must_use]
    pub fn fit_size(self, logical: f32) -> Self {
        self.fit_box(logical, logical)
    }

    fn fit_box(mut self, width: f32, height: f32) -> Self {
        self.scale = 2;
        self.fit = Some(
            LayoutStyle::block()
                .fixed_size(width, height)
                .shrink(0.0)
                .min_size(width, height),
        );
        self
    }

    #[must_use]
    pub(crate) const fn retina(mut self) -> Self {
        self.scale = 2;
        self
    }

    /// Record a vector path into a `canvas` scene.
    pub fn draw(self, scene: &mut SceneBuilder, destination: Rect, theme: &Theme) {
        let Some(source) = self.rasterize(theme) else {
            return;
        };
        let path = Arc::new(self);
        scene.draw_image(DrawImage {
            node: None,
            source,
            fit: ObjectFit::Fill,
            destination,
            opacity: 10_000,
            crop: None,
            raster: Some(path.bind(theme)),
        });
    }

    fn resolved(&self, theme: &Theme) -> ResolvedPaint {
        let fill = self.fill.as_ref().map(|fill| match fill {
            PathFill::Theme(token) => PathFill::Solid(theme.color(*token)),
            other => other.clone(),
        });
        let stroke = self
            .stroke
            .map(|(color, width)| (color.resolve(theme), width));
        let key = self.intern_key(fill.as_ref(), stroke);
        (fill, stroke, key)
    }

    fn intern_key(&self, fill: Option<&PathFill>, stroke: Option<(Rgba8, f32)>) -> u64 {
        let mut hash = OFFSET;
        for byte in self
            .width
            .to_le_bytes()
            .into_iter()
            .chain(self.height.to_le_bytes())
        {
            mix(&mut hash, byte);
        }
        mix(&mut hash, u8::from(self.non_scaling));
        for cmd in &self.cmds {
            hash_cmd(&mut hash, *cmd);
        }
        mix_stroke(&mut hash, stroke);
        mix_fill(&mut hash, fill);
        hash
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "path extents are small positive integers"
    )]
    fn rasterize(&self, theme: &Theme) -> Option<ImageSource> {
        let (fill, stroke, key) = self.resolved(theme);
        let scale = f32::from(self.scale.max(1));
        let pixels = |extent: u32| ((extent as f32) * scale).round() as u32;
        let target = (pixels(self.width), pixels(self.height), scale);
        self.raster(fill.as_ref(), stroke, key, target)
    }

    fn bind(self: &Arc<Self>, theme: &Theme) -> ImageRaster {
        let (fill, stroke, key) = self.resolved(theme);
        let path = Arc::clone(self);
        ImageRaster::new(key, move |width, height, scale| {
            path.raster(fill.as_ref(), stroke, key, (width, height, scale))
        })
    }

    fn raster(
        &self,
        fill: Option<&PathFill>,
        stroke: Option<(Rgba8, f32)>,
        mut key: u64,
        size: RasterSize,
    ) -> Option<ImageSource> {
        for byte in size.0.to_le_bytes().into_iter().chain(size.1.to_le_bytes()) {
            mix(&mut key, byte);
        }
        key::mix_f32(&mut key, size.2);
        cached_image(key, || {
            crate::profile::count(crate::profile::Counter::PathRaster);
            self.paint(fill, stroke, size)
        })
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel extents are bounded by the raster size cap"
    )]
    fn paint(
        &self,
        fill: Option<&PathFill>,
        stroke: Option<(Rgba8, f32)>,
        (width, height, scale): RasterSize,
    ) -> Option<ImageSource> {
        let path = build_ts_path(&self.cmds)?;
        let mut pixmap = Pixmap::new(width, height)?;
        let transform = Transform::from_scale(
            width as f32 / self.width.max(1) as f32,
            height as f32 / self.height.max(1) as f32,
        );
        if let Some(fill) = fill {
            let paint = Paint {
                shader: fill_shader(fill),
                anti_alias: true,
                ..Paint::default()
            };
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
        }
        if let Some((color, line)) = stroke {
            let fixed = self.non_scaling.then_some(scale);
            stroke_path(&mut pixmap, &path, (color, line), transform, fixed);
        }
        ImageSource::from_rgba8(width, height, straight_rgba(&pixmap))
    }
}

impl IntoNode for PathNode {
    fn build(mut self, cx: &mut Cx) -> NodeId {
        let overrides = std::mem::take(&mut self.layout_overrides);
        let Some(source) = self.rasterize(&Theme::light()) else {
            return overrides.finish(container()).build(cx);
        };
        let picture = match self.fit.take() {
            Some(fit) => image(source).layout(fit),
            None => image(source),
        };
        let id = overrides.finish(picture.into_node_builder()).build(cx);
        let path = Arc::new(self);
        let fallback = Arc::clone(&path);
        let themed = ThemedImage::new(
            move |theme| fallback.rasterize(theme),
            move |theme| path.bind(theme),
        );
        if let Some(node) = cx.get_mut(id) {
            let _ = node.attach(themed);
        }
        id
    }
}

builder_impls!([] PathNode: widget);

#[cfg(test)]
#[path = "vector_tests.rs"]
mod tests;
