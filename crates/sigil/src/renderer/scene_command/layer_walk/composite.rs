use crate::geometry::{Point, Rect};
use crate::renderer::filter::LayerFilters;
use crate::renderer::layer_composite::{
    Backdrop, CompositeRequest, LayerCompositePipeline, LayerMask,
};
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::projected::{ProjectedPipeline, ProjectedRequest};
use crate::renderer::scene_command::walk::{IDENTITY, WalkCtx, compose, translation};
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::command::{BlendMode, Transform3D};
use crate::scene::corner_radii::CornerRadii;
use crate::style::Rgba;

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
    if let Some(filters) = &layer.plan.backdrop {
        filter_behind(ctx, target, &request, filters);
    }
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
    release_layer(ctx, layer.handle);
}

fn filter_behind(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    request: &CompositeRequest<'_>,
    filters: &LayerFilters,
) {
    let area = request.round.expand(filters.reach());
    let Some(copy) = backdrop::copy_area(ctx, target, area, true) else {
        return;
    };
    let Ok(filtered) = super::filtered_view(ctx, &copy.handle, filters) else {
        release(ctx, Some(copy));
        return;
    };
    let behind = CompositeRequest {
        layer_view: filtered.as_ref().unwrap_or(&copy.handle.view),
        dest: copy.rect(ctx.scale),
        texture_size: (copy.handle.width, copy.handle.height),
        source_extent: copy.extent,
        ..*request
    };
    draw(ctx, target, behind, (None, None));
    release(ctx, Some(copy));
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

fn release_layer(ctx: &mut WalkCtx<'_>, handle: LayerHandle) {
    ctx.layer_pool
        .as_mut()
        .expect("layer pool acquired the texture")
        .release(handle);
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
    match layer.plan.blend {
        BlendMode::Normal => project(ctx, target, &target.view, request),
        mode => project_blended(ctx, target, request, mode),
    }
    release_layer(ctx, layer.handle);
}

fn project(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    view: &wgpu::TextureView,
    request: ProjectedRequest<'_>,
) {
    let pipeline = ProjectedPipeline::for_format(ctx.projected_pipelines, ctx.device, ctx.format);
    let paint = PaintCtx {
        device: ctx.device,
        queue: ctx.queue,
        encoder: &mut *ctx.encoder,
        view,
        viewport: target.viewport(ctx.scale),
        clip: target
            .state
            .current_clip()
            .map(|clip| local_rect(clip, target.bounds.origin)),
        scale: ctx.scale,
    };
    pipeline.composite(paint, request);
}

fn project_blended(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    request: ProjectedRequest<'_>,
    mode: BlendMode,
) {
    let texture = target.view.texture();
    let (width, height) = (texture.width(), texture.height());
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let Ok(scratch) = pool.acquire(ctx.device, width, height, texture.format()) else {
        return;
    };
    ctx.encoder.clear(&scratch.view, Rgba::TRANSPARENT);
    project(ctx, target, &scratch.view, request);
    let (w, h) = target.viewport(ctx.scale);
    let whole = Rect::new(Point::new(0.0, 0.0), w, h);
    let flat = CompositeRequest {
        layer_view: &scratch.view,
        dest: whole,
        round: whole,
        texture_size: (scratch.width, scratch.height),
        source_extent: (width, height),
        opacity: 1.0,
        clip_radii: CornerRadii::ZERO,
        transform: IDENTITY,
    };
    let copy = backdrop::copy(ctx, target, &flat);
    let behind = copy.as_ref().map(|copy| copy.backdrop(mode));
    draw(ctx, target, flat, (None, behind));
    release(ctx, copy);
    if let Some(layers) = ctx.layer_pool.as_mut() {
        layers.release(scratch);
    }
}
