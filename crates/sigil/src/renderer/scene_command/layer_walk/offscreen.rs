use crate::geometry::Rect;

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
use crate::scene::corner_radii::CornerRadii;
use crate::style::Color;

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
    const fn name(self) -> &'static str {
        match self {
            Self::RoundedClip => "rounded clip",
            Self::Opacity => "opacity",
            Self::OpacityGroup => "opacity group",
            Self::Blur => "blur",
            Self::Transform => "transform",
            Self::Other => "other",
        }
    }

    fn note(self, (width, height): (u32, u32)) {
        crate::profile::attribute(
            crate::profile::Hotspot::GpuLayer,
            u64::from(width) * u64::from(height),
            || format!("{} {width}x{height}", self.name()),
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
    blur_sigma: Option<f32>,
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
    plan: LayerPlan,
) -> Result<(OpenLayer, Target), RendererError> {
    let LayerPlan {
        reason,
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
    reason.note(extent);
    ctx.encoder.clear(&handle.view, Color::TRANSPARENT);
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
