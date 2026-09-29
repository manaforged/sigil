#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{FillRule, Mask, Pixmap, Transform as TsTransform};

use crate::renderer::frame_stats::FrameStats;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{IDENTITY, intersect_rect};
use crate::scene::corner_radii::CornerRadii;

use super::bound::{Bound, rounded_rect};
use super::glyphs::GlyphCache;
use super::raster::RasterCache;
use super::shapes::{Brush, cover, ts_transform};

pub(super) struct Masks<'a> {
    clip: Option<(Rect, Mask)>,
    pub(super) covers: &'a mut Vec<Mask>,
    pub(super) faded: Option<Mask>,
}

pub(super) struct Painter<'a> {
    pub(super) target: &'a mut Pixmap,
    pub(super) scale: f32,
    pub(super) origin: Point,
    pub(super) state: SceneState,
    pub(super) mask: Masks<'a>,
    pub(super) glyphs: &'a mut GlyphCache,
    pub(super) raster: &'a mut RasterCache,
    pub(super) stats: &'a mut FrameStats,
    pub(super) pool: &'a mut Vec<Pixmap>,
}

pub(super) struct Scratch<'a> {
    pub(super) glyphs: &'a mut GlyphCache,
    pub(super) raster: &'a mut RasterCache,
    pub(super) stats: &'a mut FrameStats,
    pub(super) pool: &'a mut Vec<Pixmap>,
    pub(super) covers: &'a mut Vec<Mask>,
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
    pub(super) fn new(target: &'a mut Pixmap, scale: f32, scratch: &'a mut Scratch<'a>) -> Self {
        Self {
            target,
            scale,
            origin: Point::new(0.0, 0.0),
            state: SceneState::default(),
            mask: Masks {
                clip: None,
                covers: &mut *scratch.covers,
                faded: None,
            },
            glyphs: &mut *scratch.glyphs,
            raster: &mut *scratch.raster,
            stats: &mut *scratch.stats,
            pool: &mut *scratch.pool,
        }
    }

    pub(super) const fn spawn<'b>(
        &'b mut self,
        target: &'b mut Pixmap,
        origin: Point,
        state: SceneState,
    ) -> Painter<'b> {
        Painter {
            target,
            scale: self.scale,
            origin,
            state,
            mask: Masks {
                clip: None,
                covers: &mut *self.mask.covers,
                faded: None,
            },
            glyphs: &mut *self.glyphs,
            raster: &mut *self.raster,
            stats: &mut *self.stats,
            pool: &mut *self.pool,
        }
    }

    pub(super) fn walk(&mut self, commands: &[SceneCommand]) {
        let mut index = 0;
        while let Some(command) = commands.get(index) {
            index += 1;
            match command {
                SceneCommand::PushLayer(scope) => {
                    let end = SceneState::layer_end(commands, index);
                    if !self.state.clipped_out() {
                        self.layer(*scope, commands.get(index..end).unwrap_or_default());
                    }
                    index = end + 1;
                }
                SceneCommand::SaveLayer { opacity } => {
                    let end = SceneState::group_end(commands, index);
                    let children = commands.get(index..end).unwrap_or_default();
                    if SceneState::composites_group(children) {
                        if !self.state.clipped_out() {
                            self.group(*opacity, children);
                        }
                        index = end + 1;
                    } else {
                        self.handle(command);
                    }
                }
                SceneCommand::PopLayer => {}
                other => self.handle(other),
            }
        }
    }

    fn handle(&mut self, command: &SceneCommand) {
        if self.state.apply(command) || self.state.clipped_out() {
            return;
        }
        match command {
            SceneCommand::PushLayer(_)
            | SceneCommand::PopLayer
            | SceneCommand::PushClip(_)
            | SceneCommand::PopClip
            | SceneCommand::PushTransform(_)
            | SceneCommand::PopTransform
            | SceneCommand::SaveLayer { .. }
            | SceneCommand::RestoreLayer => {}
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

    pub(super) fn ctx(&self) -> Ctx {
        Ctx {
            transform: ts_transform(self.state.current_transform(), self.scale, self.origin),
            clip: self.state.current_clip(),
            scale: self.scale,
            origin: self.origin,
            width: self.target.width(),
            height: self.target.height(),
        }
    }
}

pub(super) fn brush<'m>(
    masks: &'m mut Masks<'_>,
    ctx: &Ctx,
    bounds: Rect,
    opacity: f32,
) -> Option<Brush<'m>> {
    let Masks { clip, covers, .. } = masks;
    let (mask, cover) = coverage(clip, covers, ctx, bounds)?;
    Some(Brush {
        transform: ctx.transform,
        mask,
        cover,
        opacity,
    })
}

pub(super) fn shader_brush<'m>(
    masks: &'m mut Masks<'_>,
    ctx: &Ctx,
    bounds: Rect,
    opacity: f32,
) -> Option<Brush<'m>> {
    if opacity >= 1.0 {
        return brush(masks, ctx, bounds, 1.0);
    }
    let Masks {
        clip,
        covers,
        faded,
    } = masks;
    let (mask, cover) = coverage(clip, covers, ctx, bounds)?;
    let base = mask.or(cover)?;
    Some(Brush {
        transform: ctx.transform,
        mask: Some(fade(faded, base, opacity)?),
        cover: None,
        opacity: 1.0,
    })
}

pub(super) fn fade<'m>(slot: &'m mut Option<Mask>, base: &Mask, opacity: f32) -> Option<&'m Mask> {
    let level = u16::from((opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
    if level == 0 {
        return None;
    }
    if slot
        .as_ref()
        .is_none_or(|mask| mask.width() != base.width() || mask.height() != base.height())
    {
        *slot = Some(Mask::new(base.width(), base.height())?);
    }
    let out = slot.as_mut()?;
    for (value, source) in out.data_mut().iter_mut().zip(base.data()) {
        *value = ((u16::from(*source) * level + 127) / 255) as u8;
    }
    Some(out)
}

type Coverage<'m> = (Option<&'m Mask>, Option<&'m Mask>);

fn coverage<'m>(
    cache: &'m mut Option<(Rect, Mask)>,
    covers: &'m mut Vec<Mask>,
    ctx: &Ctx,
    bounds: Rect,
) -> Option<Coverage<'m>> {
    let clipped = match ctx.clip {
        Some(clip) => {
            intersect_rect(clip, bounds)?;
            (!contains(clip, bounds)).then_some(clip)
        }
        None => None,
    };
    let Some(clip) = clipped else {
        return Some((None, cover(covers, ctx.width, ctx.height)));
    };
    if cache.as_ref().is_none_or(|(rect, _)| *rect != clip) {
        let mut mask = Mask::new(ctx.width, ctx.height)?;
        let transform = ts_transform(IDENTITY, ctx.scale, ctx.origin);
        let bound = Bound::new(transform, ctx.width, ctx.height);
        let path = rounded_rect(clip, CornerRadii::ZERO, bound)?;
        mask.fill_path(&path, FillRule::Winding, true, transform);
        *cache = Some((clip, mask));
    }
    Some((cache.as_ref().map(|(_, m)| m), None))
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.origin.x + inner.width <= outer.origin.x + outer.width
        && inner.origin.y + inner.height <= outer.origin.y + outer.height
}
