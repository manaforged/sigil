use std::rc::Rc;

use crate::kernel::{
    Cx, ImageRaster, ImageSource, IntoNode, NodeId, ThemedImage, cached_image, container, image,
};
use crate::layout::LayoutStyle;
use crate::style::{Rgba8, Theme, Token};
use tiny_skia::{FillRule, Paint, Pixmap, Transform};

mod key;
mod raster;

use key::{OFFSET, hash_cmd, mix, mix_fill, mix_stroke};
use raster::{build_ts_path, fill_shader, straight_rgba, stroke_path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathColor {
    Rgba(Rgba8),
    Token(Token),
}

impl PathColor {
    fn resolve(self, theme: &Theme) -> Rgba8 {
        match self {
            Self::Rgba(color) => color,
            Self::Token(token) => theme.color(token),
        }
    }
}

impl From<Rgba8> for PathColor {
    fn from(color: Rgba8) -> Self {
        Self::Rgba(color)
    }
}

impl From<Token> for PathColor {
    fn from(token: Token) -> Self {
        Self::Token(token)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathFill {
    Solid(Rgba8),
    Theme(Token),
    Linear {
        start: (f32, f32),
        end: (f32, f32),
        stops: Vec<(f32, Rgba8)>,
    },
    Radial {
        center: (f32, f32),
        radius: f32,
        stops: Vec<(f32, Rgba8)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

pub struct PathNode {
    width: u32,
    height: u32,
    cmds: Vec<PathCmd>,
    fill: Option<PathFill>,
    stroke: Option<(PathColor, f32)>,
    layout: Option<crate::layout::LayoutStyle>,
    scale: u8,
    non_scaling: bool,
}

type RasterSize = (u32, u32, f32);

#[must_use]
pub const fn path(width: u32, height: u32) -> PathNode {
    PathNode {
        width,
        height,
        cmds: Vec::new(),
        fill: None,
        stroke: None,
        layout: None,
        scale: 1,
        non_scaling: false,
    }
}

impl PathNode {
    #[must_use]
    pub fn move_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::MoveTo(x, y));
        self
    }

    #[must_use]
    pub fn line_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::LineTo(x, y));
        self
    }

    #[must_use]
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::QuadTo(cx, cy, x, y));
        self
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    #[must_use]
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y));
        self
    }

    #[must_use]
    pub fn close(mut self) -> Self {
        self.cmds.push(PathCmd::Close);
        self
    }

    #[must_use]
    pub fn fill(mut self, color: impl Into<PathColor>) -> Self {
        self.fill = Some(match color.into() {
            PathColor::Rgba(color) => PathFill::Solid(color),
            PathColor::Token(token) => PathFill::Theme(token),
        });
        self
    }

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

    #[must_use]
    pub fn stroke(mut self, color: impl Into<PathColor>, width: f32) -> Self {
        self.stroke = Some((color.into(), width));
        self
    }

    #[must_use]
    pub const fn non_scaling_stroke(mut self) -> Self {
        self.non_scaling = true;
        self
    }

    #[must_use]
    pub fn layout(mut self, style: LayoutStyle) -> Self {
        self.layout = Some(style);
        self
    }

    #[must_use]
    pub fn fit(self) -> Self {
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let (width, height) = (self.width as f32, self.height as f32);
        self.fit_box(width, height)
    }

    #[must_use]
    pub fn fit_size(self, logical: f32) -> Self {
        self.fit_box(logical, logical)
    }

    fn fit_box(mut self, width: f32, height: f32) -> Self {
        self.scale = 2;
        self.layout = Some(
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

    fn resolved(&self, theme: &Theme) -> (Option<PathFill>, Option<(Rgba8, f32)>) {
        let fill = self.fill.as_ref().map(|fill| match fill {
            PathFill::Theme(token) => PathFill::Solid(theme.color(*token)),
            other => other.clone(),
        });
        let stroke = self
            .stroke
            .map(|(color, width)| (color.resolve(theme), width));
        (fill, stroke)
    }

    fn intern_key(&self, fill: Option<&PathFill>, stroke: Option<(Rgba8, f32)>) -> u64 {
        let mut hash = OFFSET;
        for byte in self.width.to_le_bytes() {
            mix(&mut hash, byte);
        }
        for byte in self.height.to_le_bytes() {
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
        let (fill, stroke) = self.resolved(theme);
        let key = self.intern_key(fill.as_ref(), stroke);
        let scale = f32::from(self.scale.max(1));
        let pixels = |extent: u32| ((extent as f32) * scale).round() as u32;
        let target = (pixels(self.width), pixels(self.height), scale);
        self.raster(fill.as_ref(), stroke, key, target)
    }

    fn bind(self: &Rc<Self>, theme: &Theme) -> ImageRaster {
        let (fill, stroke) = self.resolved(theme);
        let key = self.intern_key(fill.as_ref(), stroke);
        let path = Rc::clone(self);
        ImageRaster::new(key, move |width, height, scale| {
            path.raster(fill.as_ref(), stroke, key, (width, height, scale))
        })
    }

    fn raster(
        &self,
        fill: Option<&PathFill>,
        stroke: Option<(Rgba8, f32)>,
        key: u64,
        size: RasterSize,
    ) -> Option<ImageSource> {
        let mut key = key;
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
        let Some(source) = self.rasterize(&Theme::light()) else {
            return container().build(cx);
        };
        let id = image(source).build(cx);
        let layout = self.layout.take();
        let path = Rc::new(self);
        let fallback = Rc::clone(&path);
        let themed = ThemedImage::new(
            move |theme| fallback.rasterize(theme),
            move |theme| path.bind(theme),
        );
        if let Some(node) = cx.get_mut(id) {
            if let Some(style) = layout {
                let _ = node.attach(style);
            }
            let _ = node.attach(themed);
        }
        id
    }
}

#[cfg(test)]
#[path = "vector_tests.rs"]
mod tests;
