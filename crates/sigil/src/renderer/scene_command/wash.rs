#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "baked wash extents are positive pixel counts bounded by the device texture limit"
)]

use core::slice::from_ref;

use crate::geometry::{Point, Rect};
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_upload::FrameUploads;
use crate::renderer::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::renderer::scene_command::walk::{IDENTITY, SceneWalk, pipeline_for};
use crate::renderer::scene_command::{DrawPattern, PatternKind};
use crate::renderer::scene_instance::{
    BlendSpace, SceneUniforms, draw_pattern_instance, gradient_stop_records, wash_stops,
};
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::wash_cache::FORMAT;
use crate::scene::Wash;
use crate::style::Color;

impl SceneWalk<'_> {
    /// Draws a wash from its baked texture; false when it must be shaded in place.
    pub(super) fn draw_wash(&mut self, draw: &DrawPattern, wash: Wash) -> bool {
        if draw.stroke.is_some() {
            return false;
        }
        let size = (
            (draw.rect.width * self.ctx.scale).ceil() as u32,
            (draw.rect.height * self.ctx.scale).ceil() as u32,
        );
        let limit = self.ctx.device.limits().max_texture_dimension_2d;
        if size.0 == 0 || size.1 == 0 || size.0 > limit || size.1 > limit {
            return false;
        }
        self.flush_shapes();
        let wash = wash.normalized();
        let view = self.ctx.wash_cache.find(wash, size).unwrap_or_else(|| {
            let view = self.bake(wash, size);
            self.ctx.wash_cache.insert(wash, size, view.clone());
            view
        });
        self.composite_wash(draw, &view, size);
        true
    }

    fn bake(&mut self, wash: Wash, size: (u32, u32)) -> wgpu::TextureView {
        let texture = wash_texture(self.ctx.device, size);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let scale = self.ctx.scale;
        let viewport = (size.0 as f32 / scale, size.1 as f32 / scale);
        let space = BlendSpace::from_format(FORMAT);
        let draw = DrawPattern::new(
            Rect::new(Point::new(0.0, 0.0), viewport.0, viewport.1),
            PatternKind::Wash(wash),
        );
        let stops = gradient_stop_records(&wash_stops(draw.pattern).unwrap_or_default(), space);
        let count = u32::try_from(stops.len()).unwrap_or(u32::MAX);
        let instance = draw_pattern_instance(&draw, 1.0, space, [0, count]).transformed(IDENTITY);
        let ctx = &mut self.ctx;
        ctx.encoder.clear(&view, Color::rgba(0.0, 0.0, 0.0, 0.0));
        let stops = FrameUploads::upload_stops(ctx.encoder, &stops, &mut *ctx.stats);
        let instances = ctx.encoder.upload(from_ref(&instance));
        let (buffer, offset, _) = ctx.encoder.uniform(&SceneUniforms {
            viewport: viewport.into(),
            _pad: [0.0, 0.0],
        });
        let pipeline = pipeline_for(
            ctx.fill_pipelines,
            ctx.device,
            FORMAT,
            (FillPipeline::format, FillPipeline::new),
        );
        pipeline.paint_uploaded(
            PaintCtx {
                device: ctx.device,
                queue: ctx.queue,
                encoder: &mut *ctx.encoder,
                view: &view,
                viewport,
                clip: None,
                scale,
            },
            instances,
            &stops,
            wgpu::BufferBinding {
                buffer: &buffer,
                offset: u64::from(offset),
                size: wgpu::BufferSize::new(size_of::<SceneUniforms>() as u64),
            },
        );
        view
    }

    fn composite_wash(&mut self, draw: &DrawPattern, view: &wgpu::TextureView, size: (u32, u32)) {
        let clip = self.state.current_clip();
        let transform = self.state.current_transform();
        let opacity = self.state.current_opacity() * f32::from(draw.opacity) / 10_000.0;
        let pipeline = LayerCompositePipeline::for_format(
            self.ctx.layer_pipelines,
            self.ctx.device,
            self.ctx.format,
        );
        pipeline.composite(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut *self.ctx.encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
                clip,
                scale: self.ctx.scale,
            },
            CompositeRequest {
                layer_view: view,
                dest: draw.rect,
                round: draw.rect,
                texture_size: size,
                source_extent: size,
                opacity,
                clip_radii: draw.border_radius,
                transform,
            },
        );
    }
}

fn wash_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sigil-renderer baked wash"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
