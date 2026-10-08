#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::Paint;

use crate::renderer::scene_command::state::shadow_reach;
use crate::renderer::scene_command::walk::{intersect_rect, transform_rect};
use crate::renderer::scene_command::{
    DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite, GlyphRun,
    StrokeBand,
};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::corner_radii::CornerRadii;

use super::bound::{Bound, rounded_rect};
use super::glyph_outline::{fill_glyphs, run_paths};
use super::glyphs::{RunPlacement, glyph_table};
use super::paint::{Painter, brush, shader_brush};
use super::pattern::Placement;
use super::shadow::{ShadowPlacement, shadow};
use super::shapes::{self, gradient, opacity_of, solid};
use super::stroke::stroke;
use crate::renderer::glyph_place::GlyphMode;

impl Painter<'_> {
    pub(super) fn sprite(&mut self, draw: &DrawSprite) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        let bounds = transform_rect(self.state.current_transform(), draw.destination);
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            raster,
            stats,
            ..
        } = self;
        let Some(brush) = shader_brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        raster.sprite(target, draw, draw.destination, &brush);
        stats.sprite_draws += 1;
    }

    pub(super) fn image(&mut self, draw: &DrawImage) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        let bounds = transform_rect(self.state.current_transform(), draw.destination);
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            raster,
            stats,
            ..
        } = self;
        let Some(brush) = shader_brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        raster.image(target, draw, ctx.scale, &brush);
        stats.sprite_draws += 1;
    }

    pub(super) fn decoration(&mut self, draw: &DrawDecoration) {
        let opacity = self.state.current_opacity();
        let bounds = transform_rect(self.state.current_transform(), draw.band);
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
        let bounds = transform_rect(
            self.state.current_transform(),
            StrokeBand::ink(draw.stroke, draw.rect),
        );
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
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
        let bound = Bound::of(target, brush.transform);
        if let Some(path) = rounded_rect(draw.rect, draw.border_radius, bound) {
            shapes::fill(target, &path, &solid(draw.color, opacity), &brush);
        }
        if let Some(s) = draw.stroke {
            stroke(target, draw.rect, draw.border_radius, s, &brush);
        }
        stats.rectangle_instances += 1;
    }

    pub(super) fn gradient(&mut self, draw: &DrawGradient) {
        let bounds = transform_rect(
            self.state.current_transform(),
            StrokeBand::ink(draw.stroke, draw.rect),
        );
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
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
        let bound = Bound::of(target, brush.transform);
        if let (Some(path), Some(shader)) = (
            rounded_rect(draw.rect, draw.border_radius, bound),
            gradient(&draw.gradient, opacity),
        ) {
            let paint = Paint {
                shader,
                anti_alias: true,
                colorspace: shapes::BLEND_SPACE,
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
        let bounds = transform_rect(
            self.state.current_transform(),
            StrokeBand::ink(draw.stroke, draw.rect),
        );
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            raster,
            stats,
            ..
        } = self;
        let Some(brush) = shader_brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        let bound = Bound::of(target, brush.transform);
        let place = Placement {
            kind: draw.pattern,
            rect: draw.rect,
            transform: brush.transform,
            opacity: brush.opacity,
            bound,
            size: [target.width(), target.height()],
        };
        if let Some(path) = rounded_rect(draw.rect, draw.border_radius, bound)
            && let Some(shader) = raster.pattern(&place)
        {
            let paint = Paint {
                shader,
                anti_alias: true,
                colorspace: shapes::BLEND_SPACE,
                ..Paint::default()
            };
            shapes::fill(target, &path, &paint, &brush);
        }
        if let Some(s) = draw.stroke {
            stroke(target, draw.rect, draw.border_radius, s, &brush);
        }
        stats.fill_instances += 1;
    }

    pub(super) fn shadow(&mut self, rect: [f32; 4], radii: CornerRadii, s: &Shadow) {
        let logical = Rect::new(Point::new(rect[0], rect[1]), rect[2], rect[3]);
        let placed = transform_rect(self.state.current_transform(), logical);
        let bounds = placed.expand(shadow_reach(s));
        let opacity = self.state.current_opacity();
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
        let Some(coverage) = brush.coverage() else {
            return;
        };
        let place = ShadowPlacement {
            rect: placed,
            radii,
            scale: ctx.scale,
            offset: ctx.origin,
            opacity,
        };
        shadow(target, s, &place, coverage);
        stats.shadow_instances += 1;
    }

    pub(super) fn glyph_run(&mut self, run: &GlyphRun) {
        let Some(plan) = run.plan(self.state.current_transform(), self.scale) else {
            return;
        };
        let clip = self.state.current_clip();
        if clip.is_some_and(|c| intersect_rect(c, plan.bounds).is_none()) {
            return;
        }
        let opacity = opacity_of(run.opacity, self.state.current_opacity());
        match plan.mode {
            GlyphMode::Mask => {
                let place = RunPlacement {
                    origin: run.rect.origin,
                    scale: self.scale,
                    offset: self.origin,
                    clip,
                    table: glyph_table(run.color, opacity),
                };
                self.glyphs.draw_run(self.target, run, &place);
            }
            GlyphMode::Outline => {
                let paths = run_paths(run, self.state.current_transform(), self.scale);
                self.glyph_outline(run, &paths, plan.bounds, opacity);
            }
        }
        self.stats.glyph_runs += 1;
    }

    fn glyph_outline(
        &mut self,
        run: &GlyphRun,
        paths: &[tiny_skia::Path],
        bounds: Rect,
        opacity: f32,
    ) {
        let ctx = self.ctx();
        let Self { target, mask, .. } = self;
        let Some(brush) = brush(mask, &ctx, bounds, opacity) else {
            return;
        };
        fill_glyphs(target, paths, run, opacity, &brush);
    }
}
