use crate::hal::{Color, RenderedGlyph};

use crate::renderer::decoration_paint::{DecorationPipeline, decoration_instance};
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::raster::RASTER_MAX_LOGICAL_PX;
use crate::renderer::raster_paint::{RasterDrawParams, RasterTextPipeline};
use crate::renderer::scene_command::walk::{
    QueuedGlyphs, Walk, rect_from_fields, transform_point, transform_rect,
};
use crate::renderer::scene_command::{
    DrawDecoration, DrawGlyphRun, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite,
    GlyphRun, SpriteKind, SpriteSource,
};
use crate::renderer::scene_instance::{
    BlendSpace, draw_gradient_instance, draw_pattern_instance, draw_rect_instance,
    draw_shadow_instance, gradient_stop_records,
};
use crate::renderer::scene_paint::{PaintCtx, ScenePipeline};
use crate::renderer::shadow_paint::{Shadow, ShadowPipeline};
use crate::renderer::slug::{DrawParams, QueuedRun, RunFrame, SlugTextPipeline, text_tuning};
use crate::renderer::sprite_paint::SpritePipeline;

impl Walk<'_> {
    pub(super) fn queue_rect(&mut self, draw: &DrawRect) {
        let mut transformed = *draw;
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        if self.clip_rect(transformed.rect).is_none() {
            return;
        }
        let space = BlendSpace::from_format(self.ctx.format);
        let instance = draw_rect_instance(&transformed, self.current_opacity(), space);
        self.pending_rects.push(instance);
    }

    pub(super) fn flush_rects(&mut self) {
        if self.pending_rects.is_empty() {
            return;
        }
        self.ctx.stats.record_rectangles(self.pending_rects.len());
        let clip = self.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let Some(instances) = uploads.upload_rects(ctx.queue, &self.pending_rects, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("rect upload requires viewport uniform");
        let pipeline = scene_pipeline_for(ctx.scene_pipelines, ctx.device, ctx.format);
        pipeline.paint_uploaded(
            PaintCtx {
                device: ctx.device,
                queue: ctx.queue,
                encoder: &mut *ctx.encoder,
                view: ctx.view,
                viewport: ctx.viewport,
                clip,
                scale: ctx.scale,
            },
            instances,
            uniform,
        );
        self.pending_rects.clear();
    }

    pub(super) fn queue_gradient(&mut self, draw: &DrawGradient) {
        let mut transformed = draw.clone();
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        transformed.gradient.start = transform_point(self.current_transform(), draw.gradient.start);
        transformed.gradient.end = transform_point(self.current_transform(), draw.gradient.end);
        if self.clip_rect(transformed.rect).is_none() {
            return;
        }
        let space = BlendSpace::from_format(self.ctx.format);
        let stops = gradient_stop_records(&transformed.gradient.stops, space);
        let stop_offset = self.uploads.next_stop_offset(self.pending_stops.len());
        let instance = draw_gradient_instance(
            &transformed,
            self.current_opacity(),
            stop_offset,
            u32::try_from(stops.len()).unwrap_or(u32::MAX),
            space,
        );
        self.pending_stops.extend(stops);
        self.pending_fills.push(instance);
    }

    pub(super) fn flush_fills(&mut self) {
        if self.pending_fills.is_empty() {
            return;
        }
        self.ctx.stats.record_fills(self.pending_fills.len());
        let clip = self.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        uploads.upload_stops(ctx.queue, &self.pending_stops, &mut *ctx.stats);
        let Some(instances) = uploads.upload_fills(ctx.queue, &self.pending_fills, &mut *ctx.stats)
        else {
            return;
        };
        let stops = uploads.stops().expect("fill upload requires stop storage");
        let uniform = uploads
            .uniform()
            .expect("fill upload requires viewport uniform");
        let pipeline = fill_pipeline_for(ctx.fill_pipelines, ctx.device, ctx.format);
        pipeline.paint_uploaded(
            PaintCtx {
                device: ctx.device,
                queue: ctx.queue,
                encoder: &mut *ctx.encoder,
                view: ctx.view,
                viewport: ctx.viewport,
                clip,
                scale: ctx.scale,
            },
            instances,
            stops,
            uniform,
        );
        self.pending_fills.clear();
        self.pending_stops.clear();
    }

    pub(super) fn queue_pattern(&mut self, draw: &DrawPattern) {
        let mut transformed = *draw;
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        if self.clip_rect(transformed.rect).is_none() {
            return;
        }
        let space = BlendSpace::from_format(self.ctx.format);
        let instance = draw_pattern_instance(&transformed, self.current_opacity(), space);
        self.pending_fills.push(instance);
    }

    pub(super) fn queue_shadow(
        &mut self,
        rect: [f32; 4],
        radii: crate::renderer::CornerRadii,
        shadow: Shadow,
    ) {
        let surface = transform_rect(self.current_transform(), rect_from_fields(rect));
        let instance = draw_shadow_instance(surface, radii, shadow, self.current_opacity());
        if self.clip_rect(rect_from_fields(instance.rect)).is_none() {
            return;
        }
        self.pending_shadows.push(instance);
    }

    pub(super) fn flush_shadows(&mut self) {
        if self.pending_shadows.is_empty() {
            return;
        }
        self.ctx.stats.record_shadows(self.pending_shadows.len());
        let clip = self.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let Some(instances) =
            uploads.upload_shadows(ctx.queue, &self.pending_shadows, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("shadow upload requires viewport uniform");
        let pipeline = shadow_pipeline_for(ctx.shadow_pipelines, ctx.device, ctx.format);
        pipeline.paint_uploaded(
            PaintCtx {
                device: ctx.device,
                queue: ctx.queue,
                encoder: &mut *ctx.encoder,
                view: ctx.view,
                viewport: ctx.viewport,
                clip,
                scale: ctx.scale,
            },
            instances,
            uniform,
        );
        self.pending_shadows.clear();
    }

    pub(super) fn queue_decoration(&mut self, draw: &DrawDecoration) {
        let band = transform_rect(self.current_transform(), draw.band);
        if self.clip_rect(band).is_none() {
            return;
        }
        let mut transformed = *draw;
        transformed.band = band;
        let space = BlendSpace::from_format(self.ctx.format);
        let instance = decoration_instance(&transformed, self.current_opacity(), space);
        self.pending_decorations.push(instance);
    }

    pub(super) fn flush_decorations(&mut self) {
        if self.pending_decorations.is_empty() {
            return;
        }
        self.ctx
            .stats
            .record_decorations(self.pending_decorations.len());
        let clip = self.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let Some(instances) =
            uploads.upload_decorations(ctx.queue, &self.pending_decorations, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("decoration upload requires viewport uniform");
        let pipeline = decoration_pipeline_for(ctx.decoration_pipelines, ctx.device, ctx.format);
        pipeline.paint_uploaded(
            PaintCtx {
                device: ctx.device,
                queue: ctx.queue,
                encoder: &mut *ctx.encoder,
                view: ctx.view,
                viewport: ctx.viewport,
                clip,
                scale: ctx.scale,
            },
            instances,
            uniform,
        );
        self.pending_decorations.clear();
    }

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
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let pipeline = slug_pipeline_for(ctx.slug_pipelines, ctx.device, ctx.format);
        pipeline.record_runs_in_frame(
            &mut *ctx.encoder,
            &RunFrame {
                device: ctx.device,
                queue: ctx.queue,
                view: ctx.view,
                surface_width: ctx.viewport.0,
                surface_height: ctx.viewport.1,
                scale: ctx.scale,
            },
            &runs,
            Some((uploads, &mut *ctx.stats)),
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

fn decoration_pipeline_for<'a>(
    pipelines: &'a mut Vec<DecorationPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut DecorationPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(DecorationPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("decoration pipeline index was just constructed from the vec")
}

fn shadow_pipeline_for<'a>(
    pipelines: &'a mut Vec<ShadowPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut ShadowPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(ShadowPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("shadow pipeline index was just constructed from the vec")
}

fn fill_pipeline_for<'a>(
    pipelines: &'a mut Vec<FillPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut FillPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(FillPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("fill pipeline index was just constructed from the vec")
}

fn scene_pipeline_for<'a>(
    pipelines: &'a mut Vec<ScenePipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut ScenePipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(ScenePipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("scene pipeline index was just constructed from the vec")
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
