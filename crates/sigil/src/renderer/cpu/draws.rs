#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{ColorSpace, Paint, Path, Pixmap, Shader};

#[cfg(feature = "kittest")]
use crate::renderer::scene_command::DrawSprite;
use crate::renderer::scene_command::state::shadow_reach;
use crate::renderer::scene_command::walk::{intersect_rect, transform_rect};
use crate::renderer::scene_command::{
    DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, GlyphRun, Stroke, StrokeBand,
};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::corner_radii::CornerRadii;

use super::bound::{CpuBound, rounded_rect};
use super::glyph_outline::{fill_glyphs, run_paths};
use super::paint::{CpuPainter, brush, shader_brush};
use super::pattern::PatternPlacement;
use super::quad::{QuadCache, RunPlacement};
use super::shadow::{ShadowPlacement, shadow};
use super::shapes::{self, Brush, opacity_of, rgba_table, solid};
use super::stroke::stroke;
use crate::renderer::glyph_place::GlyphMode;
use crate::scene::path_raster::gradient::shader as gradient_shader;

fn shader_paint(shader: Shader<'_>, colorspace: ColorSpace) -> Paint<'_> {
    Paint {
        shader,
        anti_alias: true,
        colorspace,
        ..Paint::default()
    }
}

impl CpuPainter<'_> {
    pub(super) fn with_brush(
        &mut self,
        bounds: Rect,
        opacity: f32,
        shader: bool,
        draw: impl FnOnce(&mut Pixmap, &mut QuadCache, &Brush<'_>) -> bool,
    ) -> bool {
        let ctx = self.ctx();
        let Self {
            target,
            mask,
            quads,
            ..
        } = self;
        let made = if shader {
            shader_brush(mask, &ctx, bounds, opacity)
        } else {
            brush(mask, &ctx, bounds, opacity)
        };
        made.is_some_and(|b| draw(target, quads, &b))
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "a shape carries geometry, outline, opacity and its fill"
    )]
    fn shape(
        &mut self,
        rect: Rect,
        radii: CornerRadii,
        outline: Option<Stroke>,
        opacity: f32,
        shader: bool,
        fill: impl FnOnce(&mut Pixmap, &mut QuadCache, &Path, &Brush<'_>, CpuBound),
    ) -> bool {
        let bounds = transform_rect(
            self.state.current_transform(),
            StrokeBand::ink(outline, rect),
        );
        self.with_brush(bounds, opacity, shader, |target, quads, brush| {
            let bound = CpuBound::of(target, brush.transform);
            if let Some(path) = rounded_rect(rect, radii, bound) {
                fill(target, quads, &path, brush, bound);
            }
            if let Some(s) = outline {
                stroke(target, rect, radii, s, brush);
            }
            true
        })
    }

    fn placed(&self, rect: Rect) -> Rect {
        transform_rect(self.state.current_transform(), rect)
    }

    #[cfg(feature = "kittest")]
    pub(super) fn sprite(&mut self, draw: &DrawSprite) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        let bounds = self.placed(draw.destination);
        if self.with_brush(bounds, opacity, true, |target, quads, brush| {
            quads.sprite(target, draw, draw.destination, brush);
            true
        }) {
            self.stats.sprite_draws += 1;
        }
    }

    pub(super) fn image(&mut self, draw: &DrawImage) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        let (bounds, scale) = (self.placed(draw.destination), self.scale);
        if self.with_brush(bounds, opacity, true, |target, quads, brush| {
            quads.image(target, draw, scale, brush);
            true
        }) {
            self.stats.sprite_draws += 1;
        }
    }

    pub(super) fn decoration(&mut self, draw: &DrawDecoration) {
        let opacity = self.state.current_opacity();
        let bounds = self.placed(draw.band);
        if self.with_brush(bounds, opacity, false, |target, _, brush| {
            shapes::decoration(target, draw, brush);
            true
        }) {
            self.stats.decoration_instances += 1;
        }
    }

    pub(super) fn rect(&mut self, draw: &DrawRect) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        if self.shape(
            draw.rect,
            draw.border_radius,
            draw.stroke,
            opacity,
            false,
            |target, _, path, brush, _| {
                shapes::fill(target, path, &solid(draw.color, opacity), brush);
            },
        ) {
            self.stats.rectangle_instances += 1;
        }
    }

    pub(super) fn gradient(&mut self, draw: &DrawGradient) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        if self.shape(
            draw.rect,
            draw.border_radius,
            draw.stroke,
            opacity,
            false,
            |target, _, path, brush, _| {
                if let Some((shader, space)) = gradient_shader(&draw.gradient, opacity) {
                    shapes::fill(target, path, &shader_paint(shader, space), brush);
                }
            },
        ) {
            self.stats.fill_instances += 1;
        }
    }

    pub(super) fn pattern(&mut self, draw: &DrawPattern) {
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        if self.shape(
            draw.rect,
            draw.border_radius,
            draw.stroke,
            opacity,
            true,
            |target, quads, path, brush, bound| {
                let place = PatternPlacement {
                    kind: draw.pattern,
                    rect: draw.rect,
                    transform: brush.transform,
                    opacity: brush.opacity,
                    bound,
                    size: [target.width(), target.height()],
                };
                if let Some(shader) = quads.pattern(&place) {
                    shapes::fill(
                        target,
                        path,
                        &shader_paint(shader, shapes::BLEND_SPACE),
                        brush,
                    );
                }
            },
        ) {
            self.stats.fill_instances += 1;
        }
    }

    pub(super) fn shadow(&mut self, rect: [f32; 4], radii: CornerRadii, s: &Shadow) {
        let placed = self.placed(Rect::new(Point::new(rect[0], rect[1]), rect[2], rect[3]));
        let opacity = self.state.current_opacity();
        let place = ShadowPlacement {
            rect: placed,
            radii,
            scale: self.scale,
            offset: self.origin,
            opacity,
        };
        let bounds = placed.expand(shadow_reach(s));
        if self.with_brush(bounds, opacity, false, |target, _, brush| {
            let Some(coverage) = brush.coverage() else {
                return false;
            };
            shadow(target, s, &place, coverage);
            true
        }) {
            self.stats.shadow_instances += 1;
        }
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
                    table: rgba_table(run.color, opacity),
                };
                self.quads.draw_run(self.target, run, &place);
            }
            GlyphMode::Outline => {
                let paths = run_paths(run, self.state.current_transform(), self.scale);
                self.with_brush(plan.bounds, opacity, false, |target, _, brush| {
                    fill_glyphs(target, &paths, run, opacity, brush);
                    true
                });
            }
        }
        self.stats.glyph_runs += 1;
    }
}
