//! Per-command draw handlers for the command-scene walker.

use sigil_hal::Color;

use crate::decoration_paint::{DecorationPipeline, decoration_instance};
use crate::error::RendererError;
use crate::fill_paint::FillPipeline;
use crate::scene_command::walk::{
    Walk, rect_fields, rect_from_fields, transform_point, transform_rect,
};
use crate::scene_command::{
    DrawDecoration, DrawGlyphRun, DrawGradient, DrawPattern, DrawRect, DrawSprite, GlyphRun,
};
use crate::scene_instance::{
    draw_gradient_instance, draw_pattern_instance, draw_rect_instance, draw_shadow_instance,
    gradient_stop_records, srgb_channel_to_linear,
};
use crate::scene_paint::{PaintCtx, ScenePipeline};
use crate::shadow_paint::{Shadow, ShadowPipeline};
use crate::slug::{DrawParams, SlugTextPipeline};
use crate::sprite_paint::SpritePipeline;

impl Walk<'_> {
    pub(super) fn draw_rect(&mut self, draw: &DrawRect) {
        let mut transformed = *draw;
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        let Some(rect) = self.clip_rect(transformed.rect) else {
            return;
        };
        transformed.rect = rect;
        let instance = draw_rect_instance(&transformed, self.current_opacity());
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene rect encoder"),
            });
        let pipeline =
            scene_pipeline_for(self.ctx.scene_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_instances(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            core::slice::from_ref(&instance),
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    pub(super) fn draw_gradient(&mut self, draw: &DrawGradient) {
        let mut transformed = draw.clone();
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        transformed.gradient.start = transform_point(self.current_transform(), draw.gradient.start);
        transformed.gradient.end = transform_point(self.current_transform(), draw.gradient.end);
        let Some(rect) = self.clip_rect(transformed.rect) else {
            return;
        };
        transformed.rect = rect;
        let stops = gradient_stop_records(&transformed.gradient.stops);
        let instance = draw_gradient_instance(
            &transformed,
            self.current_opacity(),
            0,
            u32::try_from(stops.len()).unwrap_or(u32::MAX),
        );
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene gradient encoder"),
            });
        let pipeline = fill_pipeline_for(self.ctx.fill_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_instances(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            core::slice::from_ref(&instance),
            &stops,
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    pub(super) fn draw_pattern(&mut self, draw: &DrawPattern) {
        let mut transformed = *draw;
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        let Some(rect) = self.clip_rect(transformed.rect) else {
            return;
        };
        transformed.rect = rect;
        let instance = draw_pattern_instance(&transformed, self.current_opacity());
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene pattern encoder"),
            });
        let pipeline = fill_pipeline_for(self.ctx.fill_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_instances(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            core::slice::from_ref(&instance),
            &[],
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    pub(super) fn draw_shadow(
        &mut self,
        rect: [f32; 4],
        radii: crate::CornerRadii,
        shadow: Shadow,
    ) {
        let surface = transform_rect(self.current_transform(), rect_from_fields(rect));
        let mut instance = draw_shadow_instance(surface, radii, shadow, self.current_opacity());
        let Some(expanded) = self.clip_rect(rect_from_fields(instance.rect)) else {
            return;
        };
        instance.rect = rect_fields(expanded);
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene shadow encoder"),
            });
        let pipeline =
            shadow_pipeline_for(self.ctx.shadow_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_instances(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            core::slice::from_ref(&instance),
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    pub(super) fn draw_decoration(&mut self, draw: &DrawDecoration) {
        let band = transform_rect(self.current_transform(), draw.band);
        if self.clip_rect(band).is_none() {
            return;
        }
        let mut transformed = *draw;
        transformed.band = band;
        let instance = decoration_instance(&transformed, self.current_opacity());
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene decoration encoder"),
            });
        let pipeline = decoration_pipeline_for(
            self.ctx.decoration_pipelines,
            self.ctx.device,
            self.ctx.format,
        );
        pipeline.paint_instance(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            &instance,
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    pub(super) fn draw_sprite(&mut self, draw: &DrawSprite) {
        let dest = transform_rect(self.current_transform(), draw.destination);
        if self.clip_rect(dest).is_none() {
            return;
        }
        let opacity = self.current_opacity();
        let mut transformed = draw.clone();
        transformed.destination = dest;
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene sprite encoder"),
            });
        let pipeline =
            sprite_pipeline_for(self.ctx.sprite_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_sprite(
            &mut PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            &transformed,
            opacity,
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    pub(super) fn draw_glyph_run(&mut self, draw: &DrawGlyphRun) -> Result<(), RendererError> {
        let run = &draw.run;
        let rect = transform_rect(self.current_transform(), run.rect);
        if self.clip_rect(rect).is_none() {
            return Ok(());
        }
        let Some(paragraph) = run.paragraph() else {
            return Ok(());
        };
        let color = glyph_run_color(run, self.current_opacity());
        let pipeline = slug_pipeline_for(self.ctx.slug_pipelines, self.ctx.device, self.ctx.format);
        pipeline.draw_paragraph(&DrawParams {
            device: self.ctx.device,
            queue: self.ctx.queue,
            view: self.ctx.view,
            surface_width: self.ctx.viewport.0,
            surface_height: self.ctx.viewport.1,
            paragraph,
            glyphs: Some(run.positioned_glyphs()),
            origin: (rect.origin.x, rect.origin.y),
            color: Some(color),
        })
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

fn glyph_run_color(run: &GlyphRun, opacity_scale: f32) -> Color {
    let opacity = (f32::from(run.opacity) / 10_000.0) * opacity_scale;
    Color::rgba(
        srgb_channel_to_linear(run.color.r),
        srgb_channel_to_linear(run.color.g),
        srgb_channel_to_linear(run.color.b),
        (f32::from(run.color.a) / 255.0) * opacity.clamp(0.0, 1.0),
    )
}
