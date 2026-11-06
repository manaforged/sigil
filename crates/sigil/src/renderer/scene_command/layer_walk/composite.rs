use crate::renderer::layer_composite::{
    Backdrop, CompositeRequest, LayerCompositePipeline, LayerMask,
};
use crate::renderer::projected::{ProjectedPipeline, ProjectedRequest};
use crate::renderer::scene_command::walk::{WalkCtx, compose, translation};
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::command::{BlendMode, Transform3D};

use super::super::backdrop::{self, BackdropCopy};
use super::super::{LayerTarget, local_rect, mask};
use super::OffscreenLayer;

pub(in super::super) fn composite_layer(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    layer: OffscreenLayer<'_>,
) {
    if let Some(projection) = layer.plan.projection {
        composite_projected(ctx, target, layer, projection);
        return;
    }
    let mask = layer
        .plan
        .mask
        .map(|source| mask::render(ctx, target, source, (layer.plan.dest, layer.extent)));
    let request = request_of(&layer, target);
    let copy = match layer.plan.blend {
        BlendMode::Normal => None,
        mode => backdrop::copy(ctx, target, &request).map(|copy| (copy, mode)),
    };
    let input = match &mask {
        None => Some(None),
        Some(Some(mask)) => Some(Some(mask.layer_mask())),
        Some(None) => None,
    };
    if let Some(input) = input {
        let behind = copy.as_ref().map(|(copy, mode)| copy.backdrop(*mode));
        draw(ctx, target, request, (input, behind));
    }
    if let Some(Some(mask)) = mask {
        mask.release(ctx);
    }
    release(ctx, copy.map(|(copy, _)| copy));
    ctx.layer_pool
        .as_mut()
        .expect("layer pool acquired the texture")
        .release(layer.handle);
}

fn draw(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    request: CompositeRequest<'_>,
    (mask, behind): (Option<LayerMask<'_>>, Option<Backdrop<'_>>),
) {
    let origin = target.bounds.origin;
    let pipeline = LayerCompositePipeline::for_format(ctx.layer_pipelines, ctx.device, ctx.format);
    let paint = PaintCtx {
        device: ctx.device,
        queue: ctx.queue,
        encoder: &mut *ctx.encoder,
        view: &target.view,
        viewport: target.viewport(ctx.scale),
        clip: target
            .state
            .current_clip()
            .map(|clip| local_rect(clip, origin)),
        scale: ctx.scale,
    };
    match (mask, behind) {
        (mask, Some(behind)) => pipeline.composite_blended(paint, request, mask, behind),
        (Some(mask), None) => pipeline.composite_masked(paint, request, mask),
        (None, None) => pipeline.composite(paint, request),
    }
}

fn release(ctx: &mut WalkCtx<'_>, copy: Option<BackdropCopy>) {
    if let (Some(copy), Some(pool)) = (copy, ctx.layer_pool.as_mut()) {
        pool.release(copy.handle);
    }
}

fn request_of<'a>(layer: &'a OffscreenLayer<'_>, target: &LayerTarget) -> CompositeRequest<'a> {
    let origin = target.bounds.origin;
    CompositeRequest {
        layer_view: &layer.view,
        dest: local_rect(layer.plan.dest, origin),
        round: local_rect(layer.plan.round, origin),
        texture_size: (layer.handle.width, layer.handle.height),
        source_extent: layer.extent,
        opacity: layer.plan.opacity * target.state.current_opacity(),
        clip_radii: layer.plan.clip_radii,
        transform: compose(
            translation(-origin.x, -origin.y),
            compose(layer.plan.transform, translation(origin.x, origin.y)),
        ),
    }
}

fn composite_projected(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    layer: OffscreenLayer<'_>,
    projection: Transform3D,
) {
    let origin = target.bounds.origin;
    let request = ProjectedRequest {
        layer_view: &layer.view,
        dest: layer.plan.dest,
        texture_size: (layer.handle.width, layer.handle.height),
        source_extent: layer.extent,
        opacity: layer.plan.opacity * target.state.current_opacity(),
        transform: Transform3D::from(translation(-origin.x, -origin.y)).then(projection),
    };
    let pipeline = ProjectedPipeline::for_format(ctx.projected_pipelines, ctx.device, ctx.format);
    let paint = PaintCtx {
        device: ctx.device,
        queue: ctx.queue,
        encoder: &mut *ctx.encoder,
        view: &target.view,
        viewport: target.viewport(ctx.scale),
        clip: target
            .state
            .current_clip()
            .map(|clip| local_rect(clip, origin)),
        scale: ctx.scale,
    };
    pipeline.composite(paint, request);
    ctx.layer_pool
        .as_mut()
        .expect("layer pool acquired the texture")
        .release(layer.handle);
}
