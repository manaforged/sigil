use crate::hal::RenderedGlyph;
use crate::style::Color;

use crate::renderer::glyph_place::{GlyphMode, plan_glyph_run};
use crate::renderer::raster_paint::{RasterDrawParams, RasterTextPipeline};
use crate::renderer::scene_command::walk::{QueuedGlyphs, Walk, transform_rect};
use crate::renderer::scene_command::{
    DrawGlyphRun, DrawImage, DrawSprite, GlyphRun, SpriteKind, SpriteSource,
};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::slug::{DrawParams, QueuedRun, RunFrame, SlugTextPipeline};
use crate::renderer::sprite_paint::{SpritePaint, SpritePipeline};

impl Walk<'_> {
    pub(super) fn draw_sprite(&mut self, draw: &DrawSprite) {
        let transform = self.state.current_transform();
        if self
            .state
            .clip_rect(transform_rect(transform, draw.destination))
            .is_none()
        {
            return;
        }
        let paint = SpritePaint {
            opacity: self.state.current_opacity(),
            space: BlendSpace::from_format(self.ctx.format),
            transform,
        };
        let clip = self.state.current_clip();
        let pipeline =
            sprite_pipeline_for(self.ctx.sprite_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_sprite(
            &mut PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut *self.ctx.encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
                clip,
                scale: self.ctx.scale,
            },
            draw,
            paint,
        );
        self.ctx.stats.sprite_draws += 1;
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    pub(super) fn draw_image(&mut self, draw: &DrawImage) {
        let Some((source, mut region, destination)) = draw.placement_at(self.ctx.scale) else {
            return;
        };
        let packed = {
            let pipeline =
                sprite_pipeline_for(self.ctx.sprite_pipelines, self.ctx.device, self.ctx.format);
            pipeline.register_image(self.ctx.device, self.ctx.queue, &source)
        };
        if packed.is_none() {
            return;
        }
        region.origin.x += packed.x as f32;
        region.origin.y += packed.y as f32;
        let sprite = DrawSprite {
            node: draw.node,
            source: SpriteSource {
                atlas: packed.atlas,
                region,
                phase: 0.0,
            },
            kind: SpriteKind::Poly,
            destination,
            color: crate::style::Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            opacity: draw.opacity,
        };
        self.draw_sprite(&sprite);
    }

    pub(super) fn queue_glyph_run(&mut self, draw: &DrawGlyphRun) {
        let run = &draw.run;
        let transform = self.state.current_transform();
        let Some(plan) = plan_glyph_run(run, transform, self.ctx.scale) else {
            return;
        };
        let Some(bounds) = self.state.clip_rect(plan.bounds) else {
            return;
        };
        let clip = self.state.current_clip();
        let Some(paragraph) = run.paragraph() else {
            return;
        };
        self.ctx.stats.glyph_runs += 1;
        let space = BlendSpace::from_format(self.ctx.format);
        let color = glyph_run_color(run, self.state.current_opacity(), space);
        let origin = (run.rect.origin.x, run.rect.origin.y);
        let leftover = match plan.mode {
            GlyphMode::Mask => self.raster_pass(run, origin, color, clip),
            GlyphMode::Outline => None,
        };
        let glyphs: &[RenderedGlyph] = match leftover.as_deref() {
            Some([]) => return,
            Some(slice) => slice,
            None => run.positioned_glyphs(),
        };
        let ctx = &mut self.ctx;
        let pipeline = slug_pipeline_for(ctx.slug_pipelines, ctx.device, ctx.format);
        let instances = pipeline.instances_for(
            glyphs,
            &DrawParams {
                device: ctx.device,
                queue: ctx.queue,
                view: ctx.view,
                surface_width: ctx.viewport.0,
                surface_height: ctx.viewport.1,
                scale: ctx.scale,
                paragraph,
                glyphs: Some(glyphs),
                origin,
                transform,
                color: Some(color),
                clip,
            },
        );
        self.ctx.pending.glyphs.push(QueuedGlyphs {
            run: QueuedRun { instances, clip },
            bounds,
        });
    }

    fn raster_pass(
        &mut self,
        run: &GlyphRun,
        origin: (f32, f32),
        color: Color,
        clip: Option<crate::geometry::Rect>,
    ) -> Option<Vec<RenderedGlyph>> {
        let paragraph = run.paragraph()?;
        self.finish();
        let glyphs = run.positioned_glyphs();
        let ctx = &mut self.ctx;
        let pipeline = raster_pipeline_for(ctx.raster_pipelines, ctx.device, ctx.format);
        Some(pipeline.record_paragraph(
            ctx.encoder,
            &RasterDrawParams {
                device: ctx.device,
                queue: ctx.queue,
                view: ctx.view,
                surface_width: ctx.viewport.0,
                surface_height: ctx.viewport.1,
                scale: ctx.scale,
                paragraph,
                glyphs,
                origin,
                color,
                clip,
            },
        ))
    }

    pub(super) fn flush_glyphs(&mut self) {
        if self.ctx.pending.glyphs.is_empty() {
            return;
        }
        self.flush_shapes();
        let ctx = &mut self.ctx;
        let pending = &mut *ctx.pending;
        pending
            .runs
            .extend(pending.glyphs.drain(..).map(|queued| queued.run));
        let pipeline = slug_pipeline_for(ctx.slug_pipelines, ctx.device, ctx.format);
        pipeline.record_runs_in_frame(
            &mut *ctx.encoder,
            &RunFrame {
                device: ctx.device,
                view: ctx.view,
                surface_width: ctx.viewport.0,
                surface_height: ctx.viewport.1,
                scale: ctx.scale,
            },
            &ctx.pending.runs,
            Some(&mut *ctx.stats),
        );
        ctx.pending.runs.clear();
    }
}

fn raster_pipeline_for<'a>(
    pipelines: &'a mut Vec<RasterTextPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut RasterTextPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(RasterTextPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("raster pipeline index was just constructed from the vec")
}

fn slug_pipeline_for<'a>(
    pipelines: &'a mut Vec<SlugTextPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut SlugTextPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(SlugTextPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("slug pipeline index was just constructed from the vec")
}

fn sprite_pipeline_for<'a>(
    pipelines: &'a mut Vec<SpritePipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut SpritePipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(SpritePipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("sprite pipeline index was just constructed from the vec")
}

fn glyph_run_color(run: &GlyphRun, opacity_scale: f32, space: BlendSpace) -> Color {
    let opacity = (f32::from(run.opacity) / 10_000.0) * opacity_scale;
    let [r, g, b, a] = space.rgba(run.color);
    Color::rgba(r, g, b, a * opacity.clamp(0.0, 1.0))
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "exact comparison is the contract (regression pin)"
)]
#[path = "draw_handlers_tests.rs"]
mod tests;
