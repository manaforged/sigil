use crate::geometry::Rect;

use crate::Error;
use crate::renderer::blur_paint::{PassCtx, PassSrc};
use crate::renderer::filter::{FilterPipeline, LayerFilters};
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::plan::MaskSource;
use crate::renderer::projected::ProjectedFrame;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{IDENTITY, transform_rect};
use crate::renderer::scene_command::walk::{WalkCtx, pipeline_for};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};
use crate::renderer::scene_instance::LAYER_FORMAT;
use crate::scene::command::Filter;
use crate::scene::command::{BlendMode, Transform3D};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shader::Shading;
use crate::style::Rgba;

use super::kept::KeptKey;
use super::{LayerTarget, physical_extent};

#[path = "composite.rs"]
mod composite;
pub(super) use composite::composite_layer;

pub(super) struct OffscreenLayer<'n> {
    handle: LayerHandle,
    view: wgpu::TextureView,
    extent: (u32, u32),
    plan: LayerPlan<'n>,
}

#[derive(Clone, Copy)]
enum LayerReason {
    RoundedClip,
    PathClip,
    Mask,
    Opacity,
    OpacityGroup,
    Blur,
    Filter,
    Blend,
    Backdrop,
    Transform,
    Shader,
    Other,
}

impl LayerReason {
    fn note(self, (width, height): (u32, u32)) {
        let name = match self {
            Self::RoundedClip => "rounded clip",
            Self::PathClip => "path clip",
            Self::Mask => "mask",
            Self::Opacity => "opacity",
            Self::OpacityGroup => "opacity group",
            Self::Blur => "blur",
            Self::Filter => "filter",
            Self::Blend => "blend",
            Self::Backdrop => "backdrop",
            Self::Transform => "transform",
            Self::Shader => "shader",
            Self::Other => "other",
        };
        crate::profile::attribute(
            crate::profile::Hotspot::GpuLayer,
            u64::from(width) * u64::from(height),
            (name.as_ptr() as usize as u64) ^ (u64::from(width) << 40) ^ (u64::from(height) << 20),
            || format!("{name} {width}x{height}"),
        );
    }

    fn of(scope: &LayerScope) -> Self {
        match scope.effect {
            LayerEffect::Identity if scope.blend != BlendMode::Normal => Self::Blend,
            LayerEffect::Transform(_) | LayerEffect::Projected { .. } => Self::Transform,
            LayerEffect::Filter(ref filters) if matches!(**filters, [Filter::Blur(_)]) => {
                Self::Blur
            }
            LayerEffect::Filter(_) => Self::Filter,
            LayerEffect::Mask(_) => Self::Mask,
            LayerEffect::Backdrop(_) => Self::Backdrop,
            LayerEffect::Shader(_) => Self::Shader,
            LayerEffect::Identity if scope.clip_radii != CornerRadii::ZERO => Self::RoundedClip,
            LayerEffect::Identity if scope.opacity < 1.0 => Self::Opacity,
            LayerEffect::Identity => Self::Other,
        }
    }
}

pub(super) struct LayerPlan<'n> {
    reason: LayerReason,
    dest: Rect,
    state: SceneState,
    round: Rect,
    opacity: f32,
    clip_radii: CornerRadii,
    transform: Transform,
    filters: Option<LayerFilters>,
    backdrop: Option<LayerFilters>,
    mask: Option<MaskSource<'n>>,
    projection: Option<Transform3D>,
    shading: Option<Shading>,
    blend: BlendMode,
}

impl<'n> LayerPlan<'n> {
    pub(super) const fn masked(mut self, mask: Option<MaskSource<'n>>) -> Self {
        if let Some(MaskSource::Path(_)) = mask {
            self.reason = LayerReason::PathClip;
        }
        self.mask = mask;
        self
    }
}

pub(super) struct OpenLayer<'n> {
    layer: OffscreenLayer<'n>,
    parent_format: wgpu::TextureFormat,
    keep: Option<(KeptKey, &'n [SceneCommand])>,
}

impl<'n> OpenLayer<'n> {
    pub(super) fn kept(mut self, keep: Option<(KeptKey, &'n [SceneCommand])>) -> Self {
        self.keep = keep;
        self
    }
}

pub(super) fn restored(
    (handle, extent): (LayerHandle, (u32, u32)),
    plan: LayerPlan<'_>,
) -> OffscreenLayer<'_> {
    OffscreenLayer {
        view: handle.view.clone(),
        handle,
        extent,
        plan,
    }
}

fn moved_plan<'n>(scope: &LayerScope, target: &LayerTarget, scale: f32) -> Option<LayerPlan<'n>> {
    let (bounds, content, round, clip_radii, transform, projection) = match scope.effect {
        LayerEffect::Projected { .. } => {
            let frame = ProjectedFrame::new(&target.state, scope, target.bounds, scale)?;
            let bounds = frame.bounds;
            (
                bounds,
                frame.content,
                bounds,
                CornerRadii::ZERO,
                IDENTITY,
                Some(frame.composite),
            )
        }
        LayerEffect::Transform(effect) => {
            let frame = target
                .state
                .transform_frame(scope, effect, target.bounds, scale)?;
            let (clip, radii) = (frame.scope.clip, frame.scope.clip_radii);
            (
                frame.bounds,
                frame.content,
                clip,
                radii,
                frame.composite,
                None,
            )
        }
        _ => return None,
    };
    Some(LayerPlan {
        reason: LayerReason::Transform,
        dest: bounds,
        state: SceneState::new(Some(bounds), content, 1.0),
        round,
        opacity: scope.opacity,
        clip_radii,
        transform,
        filters: None,
        backdrop: None,
        mask: None,
        projection,
        shading: None,
        blend: scope.blend,
    })
}

pub(super) fn layer_plan<'n>(
    scope: &LayerScope,
    target: &LayerTarget,
    scale: f32,
) -> Option<LayerPlan<'n>> {
    if matches!(
        scope.effect,
        LayerEffect::Projected { .. } | LayerEffect::Transform(_)
    ) {
        return moved_plan(scope, target, scale);
    }
    let bounds = target.state.layer_bounds(scope, target.bounds)?;
    let transform = target.state.current_transform();
    Some(LayerPlan {
        reason: LayerReason::of(scope),
        dest: SceneState::layer_region(bounds, target.bounds, scale),
        state: SceneState::new(Some(bounds), transform, 1.0),
        round: transform_rect(transform, scope.clip),
        opacity: scope.opacity,
        clip_radii: scope.clip_radii,
        transform: IDENTITY,
        filters: LayerFilters::of(&scope.effect),
        backdrop: LayerFilters::behind(&scope.effect),
        mask: None,
        projection: None,
        shading: match scope.effect {
            LayerEffect::Shader(shading) => Some(shading),
            _ => None,
        },
        blend: scope.blend,
    })
}

pub(super) fn group_plan<'n>(
    opacity: f32,
    commands: &[SceneCommand],
    target: &LayerTarget,
    scale: f32,
) -> Option<LayerPlan<'n>> {
    let bounds = target.state.draw_bounds(commands, target.bounds, scale)?;
    Some(LayerPlan {
        reason: LayerReason::OpacityGroup,
        dest: bounds,
        state: target.state.group_state(bounds),
        round: bounds,
        opacity: opacity.clamp(0.0, 1.0),
        clip_radii: CornerRadii::ZERO,
        transform: IDENTITY,
        filters: None,
        backdrop: None,
        mask: None,
        projection: None,
        shading: None,
        blend: BlendMode::Normal,
    })
}

pub(super) fn open_layer<'n>(
    ctx: &mut WalkCtx<'_>,
    mut plan: LayerPlan<'n>,
) -> Result<(OpenLayer<'n>, LayerTarget), Error> {
    let extent = physical_extent(plan.dest, ctx.scale);
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, extent.0, extent.1, LAYER_FORMAT)?;
    ctx.stats.record_offscreen(extent.0, extent.1);
    plan.reason.note(extent);
    ctx.encoder.clear(&handle.view, Rgba::TRANSPARENT);
    let parent_format = std::mem::replace(&mut ctx.format, LAYER_FORMAT);
    let target = LayerTarget {
        view: handle.view.clone(),
        bounds: plan.dest,
        state: std::mem::take(&mut plan.state),
    };
    let layer = OffscreenLayer {
        view: handle.view.clone(),
        handle,
        extent,
        plan,
    };
    Ok((
        OpenLayer {
            layer,
            parent_format,
            keep: None,
        },
        target,
    ))
}

pub(super) fn close_layer<'n>(
    ctx: &mut WalkCtx<'_>,
    open: OpenLayer<'n>,
) -> Result<OffscreenLayer<'n>, Error> {
    ctx.format = open.parent_format;
    let mut layer = open.layer;
    if let Some(filters) = layer.plan.filters.take()
        && let Some(view) = filtered_view(ctx, &layer.handle, &filters)?
    {
        layer.view = view;
    }
    if let Some(keep) = open.keep {
        super::kept::keep(ctx, keep, &layer.view, layer.extent)?;
    }
    Ok(layer)
}

fn filtered_view(
    ctx: &mut WalkCtx<'_>,
    handle: &LayerHandle,
    filters: &LayerFilters,
) -> Result<Option<wgpu::TextureView>, Error> {
    let mut pass = PassCtx {
        device: ctx.device,
        encoder: ctx.encoder,
        retained: ctx.retained,
    };
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let pipeline = pipeline_for(
        ctx.filter_pipelines,
        ctx.device,
        handle.format,
        (FilterPipeline::format, FilterPipeline::new),
    );
    let src = PassSrc {
        view: &handle.view,
        w: handle.width,
        h: handle.height,
    };
    pipeline.apply(&mut pass, pool, src, (filters, ctx.scale))
}
