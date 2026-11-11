use crate::geometry::Rect;

use crate::Error;
use crate::renderer::cache_limits::{CacheLimits, FrameCache};
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::scene_command::walk::WalkCtx;
use crate::renderer::scene_instance::LAYER_FORMAT;
use crate::scene::command::{LayerEffect, LayerScope, SceneCommand, Transform};

use crate::renderer::plan::{MaskSource, PlanNode};

use super::offscreen::{layer_plan, open_layer, restored};
use super::{LayerFrame, LayerTarget, Nested};

#[derive(Clone, PartialEq)]
pub(crate) struct KeptKey {
    scope: LayerScope,
    transform: Transform,
    clip: Option<Rect>,
    target: Rect,
    scale: f32,
}

impl KeptKey {
    pub(super) fn of(scope: &LayerScope, target: &LayerTarget, scale: f32) -> Option<Self> {
        if matches!(
            scope.effect,
            LayerEffect::Transform(_) | LayerEffect::Projected { .. }
        ) {
            return None;
        }
        Some(Self {
            scope: scope.clone(),
            transform: target.state.current_transform(),
            clip: target.state.current_clip(),
            target: target.bounds,
            scale,
        })
    }
}

pub(crate) type KeptLayers = FrameCache<KeptKey, (LayerHandle, (u32, u32))>;

pub(super) fn restore(
    ctx: &mut WalkCtx<'_>,
    key: &KeptKey,
    commands: &[SceneCommand],
) -> Option<(LayerHandle, (u32, u32))> {
    let (kept, extent) = ctx.kept.get(key, commands)?;
    let (width, height) = (kept.width, kept.height);
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let handle = pool.acquire(ctx.device, width, height, LAYER_FORMAT).ok()?;
    let source = kept.view.texture();
    ctx.encoder
        .copy_region(source, handle.view.texture(), (0, 0, width, height));
    Some((handle, *extent))
}

pub(super) fn keep(
    ctx: &mut WalkCtx<'_>,
    (key, commands): (KeptKey, &[SceneCommand]),
    view: &wgpu::TextureView,
    extent: (u32, u32),
) -> Result<(), Error> {
    let source = view.texture();
    let (width, height) = (source.width(), source.height());
    let area = |(kept, _): &(LayerHandle, _)| u64::from(kept.width) * u64::from(kept.height);
    let need = (
        u64::from(width) * u64::from(height),
        CacheLimits::DEFAULT.kept_layer_pixels,
    );
    if !ctx.kept.admit(commands, need, area) {
        return Ok(());
    }
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let handle = pool.acquire(ctx.device, width, height, source.format())?;
    ctx.encoder
        .copy_region(source, handle.view.texture(), (0, 0, width, height));
    ctx.kept.insert(key, commands, (handle, extent));
    Ok(())
}

pub(super) fn open_scope<'n>(
    ctx: &mut WalkCtx<'_>,
    frame: &mut LayerFrame<'n>,
    (scope, mask): (&LayerScope, Option<MaskSource<'n>>),
    (children, commands): (&'n [PlanNode<'n>], &'n [SceneCommand]),
) -> Result<Option<Nested<'n>>, Error> {
    let plan = layer_plan(scope, &frame.scan_target, ctx.scale).map(|plan| plan.masked(mask));
    let key = KeptKey::of(scope, &frame.scan_target, ctx.scale);
    let Some(plan) = plan else {
        frame.layers.push(None);
        return Ok(None);
    };
    if let Some(found) = key.as_ref().and_then(|key| restore(ctx, key, commands)) {
        frame.layers.push(Some(restored(found, plan)));
        return Ok(None);
    }
    let (open, target) = open_layer(ctx, plan)?;
    let open = open.kept(key.map(|key| (key, commands)));
    let child = LayerFrame::new(ctx, children, target);
    Ok(Some(Nested { open, frame: child }))
}
