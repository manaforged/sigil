use crate::geometry::Rect;

use crate::renderer::blur_paint::{BlurPipeline, PassCtx, PassSrc};
use crate::renderer::error::RendererError;
use crate::renderer::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{
    IDENTITY, WalkCtx, compose, pipeline_for, transform_rect, translation,
};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};
use crate::renderer::scene_instance::LAYER_FORMAT;
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::corner_radii::CornerRadii;
use crate::style::Color;

use super::{Target, local_rect, physical_extent};

pub(super) struct Layer {
    handle: LayerHandle,
    view: wgpu::TextureView,
    extent: (u32, u32),
    plan: LayerPlan,
}

#[derive(Clone, Copy)]
enum LayerReason {
    RoundedClip,
    Opacity,
    OpacityGroup,
    Blur,
    Transform,
    Other,
}

impl LayerReason {
    fn note(self, (width, height): (u32, u32)) {
        let name = match self {
            Self::RoundedClip => "rounded clip",
            Self::Opacity => "opacity",
            Self::OpacityGroup => "opacity group",
            Self::Blur => "blur",
            Self::Transform => "transform",
            Self::Other => "other",
        };
        crate::profile::attribute(
            crate::profile::Hotspot::GpuLayer,
            u64::from(width) * u64::from(height),
            || format!("{name} {width}x{height}"),
        );
    }

    fn of(scope: &LayerScope) -> Self {
        match scope.effect {
            LayerEffect::Transform(_) => Self::Transform,
            LayerEffect::LayerBlur { .. } => Self::Blur,
            LayerEffect::Identity if scope.clip_radii != CornerRadii::ZERO => Self::RoundedClip,
            LayerEffect::Identity if scope.opacity < 1.0 => Self::Opacity,
            LayerEffect::Identity => Self::Other,
        }
    }
}

pub(super) struct LayerPlan {
    reason: LayerReason,
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
}

pub(super) fn layer_plan(scope: LayerScope, target: &Target, scale: f32) -> Option<LayerPlan> {
    if let LayerEffect::Transform(effect) = scope.effect {
        let frame = target
            .state
            .transform_frame(&scope, effect, target.bounds, scale)?;
        return Some(LayerPlan {
            reason: LayerReason::Transform,
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
        reason: LayerReason::of(&scope),
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
        reason: LayerReason::OpacityGroup,
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
    mut plan: LayerPlan,
) -> Result<(OpenLayer, Target), RendererError> {
    let extent = physical_extent(plan.dest, ctx.scale);
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, extent.0, extent.1, LAYER_FORMAT)
        .map_err(RendererError::Layer)?;
    ctx.stats.record_offscreen(extent.0, extent.1);
    plan.reason.note(extent);
    ctx.encoder.clear(&handle.view, Color::TRANSPARENT);
    let parent_format = std::mem::replace(&mut ctx.format, LAYER_FORMAT);
    let target = Target {
        view: handle.view.clone(),
        bounds: plan.dest,
        state: std::mem::take(&mut plan.state),
    };
    let layer = Layer {
        view: handle.view.clone(),
        handle,
        extent,
        plan,
    };
    Ok((
        OpenLayer {
            layer,
            parent_format,
        },
        target,
    ))
}

pub(super) fn close_layer(ctx: &mut WalkCtx<'_>, open: OpenLayer) -> Result<Layer, RendererError> {
    ctx.format = open.parent_format;
    let mut layer = open.layer;
    let Some(sigma) = layer.plan.blur_sigma else {
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
    let blur = pipeline_for(
        ctx.blur_pipelines,
        ctx.device,
        LAYER_FORMAT,
        (BlurPipeline::format, BlurPipeline::new),
    );
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
        },
    );
    ctx.layer_pool
        .as_mut()
        .expect("layer pool acquired the texture")
        .release(layer.handle);
}
