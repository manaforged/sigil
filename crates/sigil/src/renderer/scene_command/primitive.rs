use crate::renderer::CornerRadii;
use crate::renderer::decoration_paint::{DecorationPipeline, decoration_instance};
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_upload::FrameUploads;
use crate::renderer::scene_command::walk::{
    Walk, rect_from_fields, transform_point, transform_rect,
};
use crate::renderer::scene_command::{DrawDecoration, DrawGradient, DrawPattern, DrawRect};
use crate::renderer::scene_instance::{
    BlendSpace, draw_gradient_instance, draw_pattern_instance, draw_rect_instance,
    draw_shadow_instance, gradient_stop_records,
};
use crate::renderer::scene_paint::{PaintCtx, ScenePipeline};
use crate::renderer::shadow_paint::{Shadow, ShadowPipeline};

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
        let Some(instances) =
            FrameUploads::upload(ctx.encoder, &self.pending_rects, &mut *ctx.stats)
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
        let stop_offset = u32::try_from(self.pending_stops.len()).unwrap_or(u32::MAX);
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
        let stops = FrameUploads::upload_stops(ctx.encoder, &self.pending_stops, &mut *ctx.stats);
        let Some(instances) =
            FrameUploads::upload(ctx.encoder, &self.pending_fills, &mut *ctx.stats)
        else {
            return;
        };
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
            &stops,
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

    pub(super) fn queue_shadow(&mut self, rect: [f32; 4], radii: CornerRadii, shadow: Shadow) {
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
            FrameUploads::upload(ctx.encoder, &self.pending_shadows, &mut *ctx.stats)
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
            FrameUploads::upload(ctx.encoder, &self.pending_decorations, &mut *ctx.stats)
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
