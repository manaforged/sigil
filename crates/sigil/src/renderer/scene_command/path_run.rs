#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "texture sides fit in u16 and f32"
)]

use crate::geometry::{Point, Rect};
use crate::renderer::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::renderer::scene_command::walk::{IDENTITY, SceneWalk, compose, transform_rect};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::vector_pass::Run;
use crate::scene::command::DrawPath;
use crate::scene::corner_radii::CornerRadii;

impl SceneWalk<'_> {
    pub(super) fn queue_path(&mut self, draw: &DrawPath) {
        let current = self.state.current_transform();
        let scale = self.ctx.scale;
        let bounds = transform_rect(current, draw.bounds).expand(1.0 / scale);
        let Some(visible) = self.state.clip_rect(bounds) else {
            return;
        };
        let extent = self.ctx.view.texture().size();
        let side = |value: u32| u16::try_from(value).unwrap_or(u16::MAX);
        let run = Run {
            size: (side(extent.width), side(extent.height)),
            scale,
            visible,
            opacity: f32::from(draw.opacity) / 10_000.0 * self.state.current_opacity(),
        };
        let device = compose([[scale, 0.0, 0.0], [0.0, scale, 0.0]], current);
        self.ctx.vector.queue(draw, device, run);
    }

    pub(super) fn flush_paths(&mut self) {
        let linear = BlendSpace::from_format(self.ctx.format) == BlendSpace::Linear;
        let ctx = &mut self.ctx;
        let Some(rendered) = ctx
            .vector
            .render(ctx.device, ctx.queue, ctx.encoder, linear)
        else {
            return;
        };
        let scale = ctx.scale;
        let dest = Rect::new(
            Point::new(
                rendered.origin.0 as f32 / scale,
                rendered.origin.1 as f32 / scale,
            ),
            rendered.size.0 as f32 / scale,
            rendered.size.1 as f32 / scale,
        );
        let pipeline =
            LayerCompositePipeline::for_format(ctx.layer_pipelines, ctx.device, ctx.format);
        pipeline.composite(
            PaintCtx {
                device: ctx.device,
                queue: ctx.queue,
                encoder: &mut *ctx.encoder,
                view: ctx.view,
                viewport: ctx.viewport,
                clip: Some(rendered.bounds),
                scale,
            },
            CompositeRequest {
                layer_view: &rendered.view,
                dest,
                round: dest,
                texture_size: rendered.size,
                source_extent: rendered.size,
                opacity: 1.0,
                clip_radii: CornerRadii::ZERO,
                transform: IDENTITY,
            },
        );
        ctx.stats.sprite_draws += 1;
    }
}
