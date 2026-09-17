use crate::hal::{Color, RenderedGlyph};

use crate::renderer::raster::RASTER_MAX_LOGICAL_PX;
use crate::renderer::raster_paint::{RasterDrawParams, RasterTextPipeline};
use crate::renderer::scene_command::walk::{QueuedGlyphs, Walk, transform_rect};
use crate::renderer::scene_command::{
    DrawGlyphRun, DrawImage, DrawSprite, GlyphRun, SpriteKind, SpriteSource,
};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::slug::{DrawParams, QueuedRun, RunFrame, SlugTextPipeline, text_tuning};
use crate::renderer::sprite_paint::SpritePipeline;

impl Walk<'_> {
    pub(super) fn draw_sprite(&mut self, draw: &DrawSprite) {
        let dest = transform_rect(self.current_transform(), draw.destination);
        if self.clip_rect(dest).is_none() {
            return;
        }
        let opacity = self.current_opacity();
        let space = BlendSpace::from_format(self.ctx.format);
        let clip = self.current_clip();
        let mut transformed = draw.clone();
        transformed.destination = dest;
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
            &transformed,
            opacity,
            space,
        );
        self.ctx.stats.sprite_draws += 1;
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    pub(super) fn draw_image(&mut self, draw: &DrawImage) {
        let packed = {
            let pipeline =
                sprite_pipeline_for(self.ctx.sprite_pipelines, self.ctx.device, self.ctx.format);
            pipeline.register_image(self.ctx.device, self.ctx.queue, &draw.source)
        };
        if packed.is_none() {
            return;
        }
        let (mut region, destination) =
            fit_image(draw.fit, packed.w as f32, packed.h as f32, draw.destination);
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
        let rect = transform_rect(self.current_transform(), run.rect);
        let Some(bounds) = self.clip_rect(rect) else {
            return;
        };
        let clip = self.current_clip();
        let Some(paragraph) = run.paragraph() else {
            return;
        };
        let glyphs = run.positioned_glyphs();
        if glyphs.is_empty() {
            return;
        }
        self.ctx.stats.glyph_runs += 1;
        let space = BlendSpace::from_format(self.ctx.format);
        let color = glyph_run_color(run, self.current_opacity(), space);
        let origin = (rect.origin.x, rect.origin.y);
        let leftover = self.raster_pass(run, origin, color, clip);
        let glyphs: &[RenderedGlyph] = match leftover.as_deref() {
            Some([]) => return,
            Some(slice) => slice,
            None => glyphs,
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
                color: Some(color),
                clip,
            },
        );
        self.pending_glyphs.push(QueuedGlyphs {
            run: QueuedRun { instances, clip },
            bounds,
        });
    }

    fn raster_pass(
        &mut self,
        run: &GlyphRun,
        origin: (f32, f32),
        color: Color,
        clip: Option<crate::hal::Rect>,
    ) -> Option<Vec<RenderedGlyph>> {
        let eligible = text_tuning().raster
            && self.is_identity_transform()
            && run.style.font_size <= RASTER_MAX_LOGICAL_PX;
        if !eligible {
            return None;
        }
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
        if self.pending_glyphs.is_empty() {
            return;
        }
        self.flush_shapes();
        let runs: Vec<QueuedRun> = self
            .pending_glyphs
            .drain(..)
            .map(|queued| queued.run)
            .collect();
        let ctx = &mut self.ctx;
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
            &runs,
            Some(&mut *ctx.stats),
        );
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

fn fit_image(
    fit: crate::kernel::ObjectFit,
    iw: f32,
    ih: f32,
    dest: crate::hal::Rect,
) -> (crate::hal::Rect, crate::hal::Rect) {
    use crate::hal::{Point, Rect};
    let full_src = Rect::new(Point::new(0.0, 0.0), iw, ih);
    if iw <= 0.0 || ih <= 0.0 || dest.width <= 0.0 || dest.height <= 0.0 {
        return (full_src, dest);
    }
    match fit {
        crate::kernel::ObjectFit::Fill => (full_src, dest),
        crate::kernel::ObjectFit::Contain => {
            let scale = (dest.width / iw).min(dest.height / ih);
            let (w, h) = (iw * scale, ih * scale);
            let x = dest.origin.x + (dest.width - w) / 2.0;
            let y = dest.origin.y + (dest.height - h) / 2.0;
            (full_src, Rect::new(Point::new(x, y), w, h))
        }
        crate::kernel::ObjectFit::Cover => {
            let scale = (dest.width / iw).max(dest.height / ih);
            let (vw, vh) = (dest.width / scale, dest.height / scale);
            let sx = (iw - vw) / 2.0;
            let sy = (ih - vh) / 2.0;
            (Rect::new(Point::new(sx, sy), vw, vh), dest)
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "exact comparison is the contract (regression pin)"
)]
#[path = "draw_handlers_tests.rs"]
mod tests;
