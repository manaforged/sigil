#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::hal::{Point, Rect};
use tiny_skia::{Paint, PremultipliedColorU8, Transform as TsTransform};

use crate::renderer::scene_command::walk::{intersect_rect, transform_rect};
use crate::renderer::scene_command::{
    DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite, GlyphRun,
};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::corner_radii::CornerRadii;

use super::glyphs::RunPlacement;
use super::paint::{Painter, brush};
use super::shadow::{ShadowPlacement, shadow};
use super::shapes::{self, gradient, opacity_of, rounded_rect, solid, stroke};

impl Painter<'_> {
    pub(super) fn sprite(&mut self, draw: &DrawSprite) {
        let opacity = opacity_of(draw.opacity, self.opacity());
        let bounds = transform_rect(self.transform(), draw.destination);
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            raster,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        raster.sprite(target, draw, draw.destination, &brush);
        stats.sprite_draws += 1;
    }

    pub(super) fn image(&mut self, draw: &DrawImage) {
        let opacity = opacity_of(draw.opacity, self.opacity());
        let bounds = transform_rect(self.transform(), draw.destination);
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            raster,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        raster.image(target, draw, draw.destination, &brush);
        stats.sprite_draws += 1;
    }

    pub(super) fn decoration(&mut self, draw: &DrawDecoration) {
        let opacity = self.opacity();
        let bounds = transform_rect(self.transform(), draw.band);
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        shapes::decoration(target, draw, &brush);
        stats.decoration_instances += 1;
    }

    pub(super) fn rect(&mut self, draw: &DrawRect) {
        let bounds = transform_rect(self.transform(), draw.rect);
        let opacity = opacity_of(draw.opacity, self.opacity());
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        if let Some(path) = rounded_rect(draw.rect, draw.border_radius) {
            shapes::fill(target, &path, &solid(draw.color, opacity), &brush);
        }
        if let Some(s) = draw.stroke {
            stroke(target, draw.rect, draw.border_radius, s, &brush);
        }
        stats.rectangle_instances += 1;
    }

    pub(super) fn gradient(&mut self, draw: &DrawGradient) {
        let bounds = transform_rect(self.transform(), draw.rect);
        let opacity = opacity_of(draw.opacity, self.opacity());
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        if let (Some(path), Some(shader)) = (
            rounded_rect(draw.rect, draw.border_radius),
            gradient(&draw.gradient, opacity),
        ) {
            let paint = Paint {
                shader,
                anti_alias: true,
                ..Paint::default()
            };
            shapes::fill(target, &path, &paint, &brush);
        }
        if let Some(s) = draw.stroke {
            stroke(target, draw.rect, draw.border_radius, s, &brush);
        }
        stats.fill_instances += 1;
    }

    pub(super) fn pattern(&mut self, draw: &DrawPattern) {
        let bounds = transform_rect(self.transform(), draw.rect);
        let opacity = opacity_of(draw.opacity, self.opacity());
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            raster,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        let Some(path) = rounded_rect(draw.rect, draw.border_radius) else {
            return;
        };
        let Some(tile) = raster.tile(draw.pattern, ctx.scale) else {
            return;
        };
        let shader = tiny_skia::Pattern::new(
            tile.as_ref(),
            tiny_skia::SpreadMode::Repeat,
            tiny_skia::FilterQuality::Nearest,
            opacity,
            TsTransform::from_scale(1.0 / ctx.scale, 1.0 / ctx.scale).post_translate(
                draw.rect.origin.x + draw.rect.width / 2.0,
                draw.rect.origin.y + draw.rect.height / 2.0,
            ),
        );
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        shapes::fill(target, &path, &paint, &brush);
        if let Some(s) = draw.stroke {
            stroke(target, draw.rect, draw.border_radius, s, &brush);
        }
        stats.fill_instances += 1;
    }

    pub(super) fn shadow(&mut self, rect: [f32; 4], radii: CornerRadii, s: &Shadow) {
        let logical = Rect::new(Point::new(rect[0], rect[1]), rect[2], rect[3]);
        let placed = transform_rect(self.transform(), logical);
        let reach = s.blur.mul_add(
            3.0,
            s.spread.abs() + s.offset[0].abs().max(s.offset[1].abs()),
        );
        let bounds = Rect::new(
            Point::new(placed.origin.x - reach, placed.origin.y - reach),
            reach.mul_add(2.0, placed.width),
            reach.mul_add(2.0, placed.height),
        );
        let opacity = self.opacity();
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            stats,
            ..
        } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        let place = ShadowPlacement {
            rect: placed,
            radii,
            scale: ctx.scale,
            offset: ctx.origin,
            opacity,
        };
        shadow(target, s, &place, brush.mask);
        stats.shadow_instances += 1;
    }

    pub(super) fn glyph_run(&mut self, run: &GlyphRun) {
        let rect = transform_rect(self.transform(), run.rect);
        let clip = self.clip();
        if clip.is_some_and(|c| intersect_rect(c, rect).is_none()) {
            return;
        }
        let opacity = opacity_of(run.opacity, self.opacity());
        let a = (f32::from(run.color.a) / 255.0 * opacity).clamp(0.0, 1.0);
        let premul = |c: u8| (f32::from(c) * a).round() as u8;
        let Some(color) = PremultipliedColorU8::from_rgba(
            premul(run.color.r),
            premul(run.color.g),
            premul(run.color.b),
            (a * 255.0).round() as u8,
        ) else {
            return;
        };
        let place = RunPlacement {
            origin: rect.origin,
            scale: self.scale,
            offset: self.origin,
            clip,
            color,
        };
        self.glyphs.draw_run(self.target, run, &place);
        self.stats.glyph_runs += 1;
    }
}
