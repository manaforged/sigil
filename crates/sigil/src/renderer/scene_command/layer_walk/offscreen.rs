use crate::hal::Rect;

use crate::renderer::blur_paint::{PassCtx, PassSrc};
use crate::renderer::error::RendererError;
use crate::renderer::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{
    IDENTITY, WalkCtx, compose, transform_rect, translation,
};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};
use crate::renderer::scene_instance::LAYER_FORMAT;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::split_encoder::SplitEncoder;
use crate::scene::corner_radii::CornerRadii;

use super::Target;
use super::geometry::{local_rect, physical_extent};
use super::pipelines::blur_pipeline_for;

pub(super) struct Layer {
    handle: LayerHandle,
    view: wgpu::TextureView,
    extent: (u32, u32),
    dest: Rect,
    round: Rect,
    opacity: f32,
    clip_radii: CornerRadii,
    transform: Transform,
}

pub(super) struct LayerPlan {
    dest: Rect,
    state: SceneState,
    round: Rect,
    opacity: f32,
    clip_radii: CornerRadii,
    transform: Transform,
    blur_sigma: Option<f32>,
}

pub(super) struct OpenLayer {
    layer: Layer,
    parent_format: wgpu::TextureFormat,
    blur_sigma: Option<f32>,
}

pub(super) fn layer_plan(scope: LayerScope, target: &Target, scale: f32) -> Option<LayerPlan> {
    if let LayerEffect::Transform(effect) = scope.effect {
        let frame = target
            .state
            .transform_frame(&scope, effect, target.bounds, scale)?;
        return Some(LayerPlan {
            dest: frame.bounds,
            state: SceneState::new(Some(frame.bounds), frame.content, 1.0),
            round: frame.scope.clip,
            opacity: scope.opacity,
            clip_radii: frame.scope.clip_radii,
            transform: frame.composite,
            blur_sigma: None,
        });
    }
    let bounds = target.state.layer_bounds(&scope, target.bounds)?;
    let transform = target.state.current_transform();
    Some(LayerPlan {
        dest: SceneState::layer_region(bounds, target.bounds, scale),
        state: SceneState::new(Some(bounds), transform, 1.0),
        round: transform_rect(transform, scope.clip),
        opacity: scope.opacity,
        clip_radii: scope.clip_radii,
        transform: IDENTITY,
        blur_sigma: match scope.effect {
            LayerEffect::LayerBlur { sigma } => Some(sigma),
            LayerEffect::Identity | LayerEffect::Transform(_) => None,
        },
    })
}

pub(super) fn group_plan(
    opacity: f32,
    commands: &[SceneCommand],
    target: &Target,
    scale: f32,
) -> Option<LayerPlan> {
    let bounds = target.state.draw_bounds(commands, target.bounds, scale)?;
    Some(LayerPlan {
        dest: bounds,
        state: target.state.group_state(bounds),
        round: bounds,
        opacity: opacity.clamp(0.0, 1.0),
        clip_radii: CornerRadii::ZERO,
        transform: IDENTITY,
        blur_sigma: None,
    })
}

pub(super) fn open_layer(
    ctx: &mut WalkCtx<'_>,
    plan: LayerPlan,
) -> Result<(OpenLayer, Target), RendererError> {
    let LayerPlan {
        dest,
        state,
        round,
        opacity,
        clip_radii,
        transform,
        blur_sigma,
    } = plan;
    let extent = physical_extent(dest, ctx.scale);
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, extent.0, extent.1, LAYER_FORMAT)
        .map_err(RendererError::Layer)?;
    ctx.stats.record_offscreen(extent.0, extent.1);
    clear_view(ctx.encoder, &handle.view);
    let parent_format = std::mem::replace(&mut ctx.format, LAYER_FORMAT);
    let target = Target {
        view: handle.view.clone(),
        bounds: dest,
        state,
    };
    let layer = Layer {
        view: handle.view.clone(),
        handle,
        extent,
        dest,
        round,
        opacity,
        clip_radii,
        transform,
    };
    let open = OpenLayer {
        layer,
        parent_format,
        blur_sigma,
    };
    Ok((open, target))
}

pub(super) fn close_layer(ctx: &mut WalkCtx<'_>, open: OpenLayer) -> Result<Layer, RendererError> {
    let OpenLayer {
        mut layer,
        parent_format,
        blur_sigma,
    } = open;
    ctx.format = parent_format;
    let Some(sigma) = blur_sigma else {
        return Ok(layer);
    };
    if let Some(view) = blurred_view(ctx, &layer.handle, sigma * ctx.scale)? {
        layer.view = view;
    }
    Ok(layer)
}

fn blurred_view(
    ctx: &mut WalkCtx<'_>,
    handle: &LayerHandle,
    sigma: f32,
) -> Result<Option<wgpu::TextureView>, RendererError> {
    let mut pass = PassCtx {
        device: ctx.device,
        encoder: ctx.encoder,
        retained: ctx.retained,
    };
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let blur = blur_pipeline_for(ctx.blur_pipelines, ctx.device, LAYER_FORMAT);
    let src = PassSrc {
        view: &handle.view,
        w: handle.width,
        h: handle.height,
    };
    blur.apply(&mut pass, pool, src, sigma)
}

pub(super) fn composite_layer(ctx: &mut WalkCtx<'_>, target: &Target, layer: Layer) {
    let origin = target.bounds.origin;
    let pipeline = LayerCompositePipeline::for_format(ctx.layer_pipelines, ctx.device, ctx.format);
    pipeline.composite(
        PaintCtx {
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
        },
        CompositeRequest {
            layer_view: &layer.view,
            dest: local_rect(layer.dest, origin),
            round: local_rect(layer.round, origin),
            texture_size: (layer.handle.width, layer.handle.height),
            source_extent: layer.extent,
            opacity: layer.opacity * target.state.current_opacity(),
            clip_radii: layer.clip_radii,
            transform: compose(
                translation(-origin.x, -origin.y),
                compose(layer.transform, translation(origin.x, origin.y)),
            ),
        },
    );
    ctx.layer_pool
        .as_mut()
        .expect("layer pool acquired the texture")
        .release(layer.handle);
}

fn clear_view(encoder: &mut SplitEncoder, view: &wgpu::TextureView) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("sigil-renderer layer clear pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}
