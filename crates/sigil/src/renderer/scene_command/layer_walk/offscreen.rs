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

use super::geometry::{local_rect, physical_extent};
use super::pipelines::blur_pipeline_for;
use super::tree::Node;
use super::{Target, render_nodes};

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

pub(super) fn render_layer(
    ctx: &mut WalkCtx<'_>,
    scope: LayerScope,
    target: &Target<'_>,
    children: &[Node<'_>],
) -> Result<Option<Layer>, RendererError> {
    if let LayerEffect::Transform(effect) = scope.effect {
        let Some(frame) = target
            .state
            .transform_frame(&scope, effect, target.bounds, ctx.scale)
        else {
            return Ok(None);
        };
        let state = SceneState::new(Some(frame.bounds), frame.content, 1.0);
        let (handle, extent) = render_offscreen(ctx, frame.bounds, state, children)?;
        return Ok(Some(Layer {
            view: handle.view.clone(),
            handle,
            extent,
            dest: frame.bounds,
            round: frame.scope.clip,
            opacity: scope.opacity,
            clip_radii: frame.scope.clip_radii,
            transform: frame.composite,
        }));
    }
    let Some(bounds) = target.state.layer_bounds(&scope, target.bounds) else {
        return Ok(None);
    };
    let transform = target.state.current_transform();
    let state = SceneState::new(Some(bounds), transform, 1.0);
    let region = SceneState::layer_region(bounds, target.bounds, ctx.scale);
    let (handle, extent) = render_offscreen(ctx, region, state, children)?;
    let blurred = match scope.effect {
        LayerEffect::LayerBlur { sigma } => blurred_view(ctx, &handle, sigma * ctx.scale)?,
        LayerEffect::Identity | LayerEffect::Transform(_) => None,
    };
    Ok(Some(Layer {
        view: blurred.unwrap_or_else(|| handle.view.clone()),
        handle,
        extent,
        dest: region,
        round: transform_rect(transform, scope.clip),
        opacity: scope.opacity,
        clip_radii: scope.clip_radii,
        transform: IDENTITY,
    }))
}

pub(super) fn render_group(
    ctx: &mut WalkCtx<'_>,
    opacity: f32,
    target: &Target<'_>,
    commands: &[SceneCommand],
    children: &[Node<'_>],
) -> Result<Option<Layer>, RendererError> {
    let Some(bounds) = target.state.draw_bounds(commands, target.bounds, ctx.scale) else {
        return Ok(None);
    };
    let state = target.state.group_state(bounds);
    let (handle, extent) = render_offscreen(ctx, bounds, state, children)?;
    Ok(Some(Layer {
        view: handle.view.clone(),
        handle,
        extent,
        dest: bounds,
        round: bounds,
        opacity: opacity.clamp(0.0, 1.0),
        clip_radii: CornerRadii::ZERO,
        transform: IDENTITY,
    }))
}

fn render_offscreen(
    ctx: &mut WalkCtx<'_>,
    bounds: Rect,
    state: SceneState,
    children: &[Node<'_>],
) -> Result<(LayerHandle, (u32, u32)), RendererError> {
    let extent = physical_extent(bounds, ctx.scale);
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, extent.0, extent.1, LAYER_FORMAT)
        .map_err(RendererError::Layer)?;
    ctx.stats.record_offscreen(extent.0, extent.1);
    clear_view(ctx.encoder, &handle.view);
    let parent = std::mem::replace(&mut ctx.format, LAYER_FORMAT);
    let rendered = render_nodes(
        ctx,
        children,
        &Target {
            view: &handle.view,
            bounds,
            state,
        },
    );
    ctx.format = parent;
    rendered?;
    Ok((handle, extent))
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

pub(super) fn composite_layer(ctx: &mut WalkCtx<'_>, target: &Target<'_>, layer: Layer) {
    let origin = target.bounds.origin;
    let pipeline = LayerCompositePipeline::for_format(ctx.layer_pipelines, ctx.device, ctx.format);
    pipeline.composite(
        PaintCtx {
            device: ctx.device,
            queue: ctx.queue,
            encoder: &mut *ctx.encoder,
            view: target.view,
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
