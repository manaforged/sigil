use crate::geometry::{Point, Rect};
use crate::renderer::decoration_paint::{DecorationPipeline, decoration_instance};
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_upload::FrameUploads;
use crate::renderer::scene_command::walk::{Walk, pipeline_for, rect_from_fields, transform_rect};
use crate::renderer::scene_command::{
    DrawDecoration, DrawGradient, DrawPattern, DrawRect, Stroke, StrokeBand, Transform,
};
use crate::renderer::scene_instance::{
    BlendSpace, draw_gradient_instance, draw_pattern_instance, draw_rect_instance,
    draw_shadow_instance, gradient_stop_records,
};
use crate::renderer::scene_paint::{PaintCtx, ScenePipeline};
use crate::renderer::shadow_paint::{Shadow, ShadowPipeline};
use crate::scene::CornerRadii;
use crate::scene::command::trim::{Trimmed, tile_phase, trim_to_view};

const TRIM_PAD: f32 = 2.0;

impl Walk<'_> {
    pub(super) fn queue_rect(&mut self, draw: &DrawRect) {
        let Some((transform, trimmed, dash)) =
            self.trimmed(draw.rect, draw.border_radius, draw.stroke)
        else {
            return;
        };
        let mut local = *draw;
        local.rect = trimmed.rect;
        local.border_radius = trimmed.radii;
        let space = BlendSpace::from_format(self.ctx.format);
        let instance = draw_rect_instance(&local, self.state.current_opacity(), space);
        self.ctx
            .pending
            .rects
            .push(instance.with_dash_origin(dash).transformed(transform));
    }

    pub(super) fn flush_rects(&mut self) {
        if self.ctx.pending.rects.is_empty() {
            return;
        }
        self.ctx
            .stats
            .record_rectangles(self.ctx.pending.rects.len());
        let clip = self.state.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let Some(instances) =
            FrameUploads::upload(ctx.encoder, &ctx.pending.rects, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("rect upload requires viewport uniform");
        let pipeline = pipeline_for(
            ctx.scene_pipelines,
            ctx.device,
            ctx.format,
            (ScenePipeline::format, ScenePipeline::new),
        );
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
        self.ctx.pending.rects.clear();
    }

    pub(super) fn queue_gradient(&mut self, draw: &DrawGradient) {
        let Some((transform, trimmed, dash)) =
            self.trimmed(draw.rect, draw.border_radius, draw.stroke)
        else {
            return;
        };
        let mut transformed = draw.clone();
        transformed.rect = trimmed.rect;
        transformed.border_radius = trimmed.radii;
        transformed.gradient = draw.gradient.normalized();
        let space = BlendSpace::from_format(self.ctx.format);
        let stops = gradient_stop_records(&transformed.gradient.stops, space);
        let stop_offset = u32::try_from(self.ctx.pending.stops.len()).unwrap_or(u32::MAX);
        let instance = draw_gradient_instance(
            &transformed,
            self.state.current_opacity(),
            stop_offset,
            u32::try_from(stops.len()).unwrap_or(u32::MAX),
            space,
        );
        self.ctx.pending.stops.extend(stops);
        self.ctx
            .pending
            .fills
            .push(instance.with_dash_origin(dash).transformed(transform));
    }

    pub(super) fn flush_fills(&mut self) {
        if self.ctx.pending.fills.is_empty() {
            return;
        }
        self.ctx.stats.record_fills(self.ctx.pending.fills.len());
        let clip = self.state.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let stops = FrameUploads::upload_stops(ctx.encoder, &ctx.pending.stops, &mut *ctx.stats);
        let Some(instances) =
            FrameUploads::upload(ctx.encoder, &ctx.pending.fills, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("fill upload requires viewport uniform");
        let pipeline = pipeline_for(
            ctx.fill_pipelines,
            ctx.device,
            ctx.format,
            (FillPipeline::format, FillPipeline::new),
        );
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
        self.ctx.pending.fills.clear();
        self.ctx.pending.stops.clear();
    }

    pub(super) fn queue_pattern(&mut self, draw: &DrawPattern) {
        let Some((transform, trimmed, dash)) =
            self.trimmed(draw.rect, draw.border_radius, draw.stroke)
        else {
            return;
        };
        let mut local = *draw;
        local.rect = trimmed.rect;
        local.border_radius = trimmed.radii;
        let phase = tile_phase(trimmed.shift, draw.pattern.tile_extent());
        let space = BlendSpace::from_format(self.ctx.format);
        let instance = draw_pattern_instance(&local, self.state.current_opacity(), space);
        self.ctx.pending.fills.push(
            instance
                .with_phase(phase)
                .with_dash_origin(dash)
                .transformed(transform),
        );
    }

    fn trimmed(
        &self,
        rect: Rect,
        radii: CornerRadii,
        stroke: Option<Stroke>,
    ) -> Option<(Transform, Trimmed, [f32; 2])> {
        let transform = self.state.current_transform();
        self.state
            .clip_rect(transform_rect(transform, StrokeBand::ink(stroke, rect)))?;
        let band = stroke.and_then(|s| StrokeBand::of(s, rect));
        let margin = band.map_or(0.0, |b| b.inside + b.outside) + TRIM_PAD;
        let (width, height) = self.ctx.viewport;
        let area = Rect::new(Point::new(0.0, 0.0), width, height);
        let trimmed = match self.state.local_view(area) {
            Some(view) => trim_to_view(rect, radii, view, margin)?,
            None => Trimmed::whole(rect, radii),
        };
        Some((transform, trimmed, trimmed.dash_origin(rect, band)))
    }

    pub(super) fn queue_shadow(&mut self, rect: [f32; 4], radii: CornerRadii, shadow: Shadow) {
        let surface = transform_rect(self.state.current_transform(), rect_from_fields(rect));
        let instance = draw_shadow_instance(surface, radii, shadow, self.state.current_opacity());
        if self
            .state
            .clip_rect(rect_from_fields(instance.rect))
            .is_none()
        {
            return;
        }
        self.ctx.pending.shadows.push(instance);
    }

    pub(super) fn flush_shadows(&mut self) {
        if self.ctx.pending.shadows.is_empty() {
            return;
        }
        self.ctx
            .stats
            .record_shadows(self.ctx.pending.shadows.len());
        let clip = self.state.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let Some(instances) =
            FrameUploads::upload(ctx.encoder, &ctx.pending.shadows, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("shadow upload requires viewport uniform");
        let pipeline = pipeline_for(
            ctx.shadow_pipelines,
            ctx.device,
            ctx.format,
            (ShadowPipeline::format, ShadowPipeline::new),
        );
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
        self.ctx.pending.shadows.clear();
    }

    pub(super) fn queue_decoration(&mut self, draw: &DrawDecoration) {
        let transform = self.state.current_transform();
        if self
            .state
            .clip_rect(transform_rect(transform, draw.band))
            .is_none()
        {
            return;
        }
        let space = BlendSpace::from_format(self.ctx.format);
        let opacity = self.state.current_opacity();
        if let Some(instance) = decoration_instance(draw, transform, opacity, space) {
            self.ctx.pending.decorations.push(instance);
        }
    }

    pub(super) fn flush_decorations(&mut self) {
        if self.ctx.pending.decorations.is_empty() {
            return;
        }
        self.ctx
            .stats
            .record_decorations(self.ctx.pending.decorations.len());
        let clip = self.state.current_clip();
        let (uploads, ctx) = (&mut self.uploads, &mut self.ctx);
        let Some(instances) =
            FrameUploads::upload(ctx.encoder, &ctx.pending.decorations, &mut *ctx.stats)
        else {
            return;
        };
        let uniform = uploads
            .uniform()
            .expect("decoration upload requires viewport uniform");
        let pipeline = pipeline_for(
            ctx.decoration_pipelines,
            ctx.device,
            ctx.format,
            (DecorationPipeline::format, DecorationPipeline::new),
        );
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
        self.ctx.pending.decorations.clear();
    }
}
