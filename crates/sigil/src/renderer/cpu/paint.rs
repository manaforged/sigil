#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::hal::{Point, Rect};
use tiny_skia::{FillRule, Mask, Pixmap, PixmapPaint, Transform as TsTransform};

use crate::renderer::corner_radii::CornerRadii;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::scene_command::walk::{IDENTITY, compose, intersect_rect, transform_rect};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};

use super::glyphs::GlyphCache;
use super::raster::RasterCache;
use super::shadow::blur;
use super::shapes::{Brush, rounded_rect, ts_transform};

pub(super) type ClipMask = Option<(Rect, Mask)>;

pub(super) struct Painter<'a> {
    pub(super) target: &'a mut Pixmap,
    pub(super) scale: f32,
    pub(super) origin: Point,
    pub(super) clips: Vec<Rect>,
    pub(super) transforms: Vec<Transform>,
    pub(super) opacities: Vec<f32>,
    pub(super) mask: ClipMask,
    pub(super) glyphs: &'a mut GlyphCache,
    pub(super) raster: &'a mut RasterCache,
    pub(super) stats: &'a mut FrameStats,
}

pub(super) struct Ctx {
    pub(super) transform: TsTransform,
    pub(super) clip: Option<Rect>,
    pub(super) scale: f32,
    pub(super) origin: Point,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl<'a> Painter<'a> {
    pub(super) fn new(
        target: &'a mut Pixmap,
        scale: f32,
        glyphs: &'a mut GlyphCache,
        raster: &'a mut RasterCache,
        stats: &'a mut FrameStats,
    ) -> Self {
        Self {
            target,
            scale,
            origin: Point::new(0.0, 0.0),
            clips: Vec::new(),
            transforms: vec![IDENTITY],
            opacities: vec![1.0],
            mask: None,
            glyphs,
            raster,
            stats,
        }
    }

    pub(super) fn walk(&mut self, commands: &[SceneCommand]) {
        let mut index = 0;
        while let Some(command) = commands.get(index) {
            index += 1;
            match command {
                SceneCommand::PushLayer(scope) => {
                    let end = layer_end(commands, index);
                    self.layer(*scope, commands.get(index..end).unwrap_or_default());
                    index = end + 1;
                }
                SceneCommand::PopLayer => {}
                other => self.handle(other),
            }
        }
    }

    fn handle(&mut self, command: &SceneCommand) {
        match command {
            SceneCommand::PushClip(rect) => {
                self.clips.push(transform_rect(self.transform(), *rect));
            }
            SceneCommand::PopClip => {
                self.clips.pop();
            }
            SceneCommand::PushTransform(t) => self.transforms.push(compose(self.transform(), *t)),
            SceneCommand::PopTransform => {
                if self.transforms.len() > 1 {
                    self.transforms.pop();
                }
            }
            SceneCommand::SaveLayer { opacity } => {
                self.opacities
                    .push(self.opacity() * opacity.clamp(0.0, 1.0));
            }
            SceneCommand::RestoreLayer => {
                if self.opacities.len() > 1 {
                    self.opacities.pop();
                }
            }
            SceneCommand::PushLayer(_) | SceneCommand::PopLayer => {}
            SceneCommand::Rect(draw) => self.rect(draw),
            SceneCommand::Gradient(draw) => self.gradient(draw),
            SceneCommand::Pattern(draw) => self.pattern(draw),
            SceneCommand::DrawShadow {
                rect,
                radii,
                shadow: s,
            } => self.shadow(*rect, *radii, s),
            SceneCommand::GlyphRun(draw) => self.glyph_run(&draw.run),
            SceneCommand::Sprite(draw) => self.sprite(draw),
            SceneCommand::Image(draw) => self.image(draw),
            SceneCommand::Decoration(draw) => self.decoration(draw),
        }
    }

    pub(super) fn transform(&self) -> Transform {
        self.transforms.last().copied().unwrap_or(IDENTITY)
    }

    pub(super) fn opacity(&self) -> f32 {
        self.opacities.last().copied().unwrap_or(1.0)
    }

    pub(super) fn clip(&self) -> Option<Rect> {
        let mut clips = self.clips.iter().copied();
        let first = clips.next()?;
        clips.try_fold(first, intersect_rect)
    }

    pub(super) fn ctx(&self) -> Ctx {
        Ctx {
            transform: ts_transform(self.transform(), self.scale, self.origin),
            clip: self.clip(),
            scale: self.scale,
            origin: self.origin,
            width: self.target.width(),
            height: self.target.height(),
        }
    }

    fn layer(&mut self, scope: LayerScope, children: &[SceneCommand]) {
        let clip_logical = transform_rect(self.transform(), scope.clip);
        let Some(bounds) = self
            .clip()
            .map_or(Some(clip_logical), |c| intersect_rect(c, clip_logical))
        else {
            return;
        };
        let x0 = bounds.origin.x.mul_add(self.scale, -self.origin.x).floor();
        let y0 = bounds.origin.y.mul_add(self.scale, -self.origin.y).floor();
        let x1 = (bounds.origin.x + bounds.width)
            .mul_add(self.scale, -self.origin.x)
            .ceil();
        let y1 = (bounds.origin.y + bounds.height)
            .mul_add(self.scale, -self.origin.y)
            .ceil();
        let Some(mut layer) = Pixmap::new((x1 - x0).max(1.0) as u32, (y1 - y0).max(1.0) as u32)
        else {
            return;
        };
        Painter {
            target: &mut layer,
            scale: self.scale,
            origin: Point::new(self.origin.x + x0, self.origin.y + y0),
            clips: vec![bounds],
            transforms: vec![self.transform()],
            opacities: vec![1.0],
            mask: None,
            glyphs: &mut *self.glyphs,
            raster: &mut *self.raster,
            stats: &mut *self.stats,
        }
        .walk(children);
        if let LayerEffect::LayerBlur { sigma } = scope.effect {
            blur(&mut layer, sigma * self.scale);
        }
        let paint = PixmapPaint {
            opacity: (scope.opacity * self.opacity()).clamp(0.0, 1.0),
            ..PixmapPaint::default()
        };
        let mask = self.layer_mask(clip_logical, scope.clip_radii);
        self.target.draw_pixmap(
            x0 as i32,
            y0 as i32,
            layer.as_ref(),
            &paint,
            TsTransform::identity(),
            mask.as_ref(),
        );
        self.stats.offscreen_layers += 1;
    }

    fn layer_mask(&self, clip: Rect, radii: CornerRadii) -> Option<Mask> {
        if radii.0.iter().all(|r| *r <= 0.0) {
            return None;
        }
        let path = rounded_rect(clip, radii)?;
        let mut mask = Mask::new(self.target.width(), self.target.height())?;
        mask.fill_path(
            &path,
            FillRule::Winding,
            true,
            ts_transform(IDENTITY, self.scale, self.origin),
        );
        Some(mask)
    }
}

pub(super) fn brush<'m>(
    cache: &'m mut ClipMask,
    ctx: &Ctx,
    bounds: Rect,
    opacity: f32,
) -> Option<Brush<'m>> {
    let transform = ctx.transform;
    let Some(clip) = ctx.clip else {
        return Some(Brush {
            transform,
            mask: None,
            opacity,
        });
    };
    intersect_rect(clip, bounds)?;
    if contains(clip, bounds) {
        return Some(Brush {
            transform,
            mask: None,
            opacity,
        });
    }
    if cache.as_ref().is_none_or(|(rect, _)| *rect != clip) {
        let mut mask = Mask::new(ctx.width, ctx.height)?;
        let path = rounded_rect(clip, CornerRadii::ZERO)?;
        mask.fill_path(
            &path,
            FillRule::Winding,
            true,
            ts_transform(IDENTITY, ctx.scale, ctx.origin),
        );
        *cache = Some((clip, mask));
    }
    Some(Brush {
        transform,
        mask: cache.as_ref().map(|(_, m)| m),
        opacity,
    })
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.origin.x + inner.width <= outer.origin.x + outer.width
        && inner.origin.y + inner.height <= outer.origin.y + outer.height
}

fn layer_end(commands: &[SceneCommand], start: usize) -> usize {
    let mut depth = 0_usize;
    for (offset, command) in commands.iter().enumerate().skip(start) {
        match command {
            SceneCommand::PushLayer(_) => depth += 1,
            SceneCommand::PopLayer if depth == 0 => return offset,
            SceneCommand::PopLayer => depth -= 1,
            _ => {}
        }
    }
    commands.len()
}
