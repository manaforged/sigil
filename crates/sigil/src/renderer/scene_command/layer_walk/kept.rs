use crate::geometry::Rect;

use crate::Error;
use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::scene_command::walk::WalkCtx;
use crate::renderer::scene_instance::LAYER_FORMAT;
use crate::scene::command::{LayerEffect, LayerScope, SceneCommand, Transform};

use crate::renderer::plan::{MaskSource, PlanNode};

use super::offscreen::{layer_plan, open_layer, restored};
use super::{LayerFrame, LayerTarget, Nested};

#[derive(Clone, PartialEq)]
pub(super) struct KeptKey {
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

struct Entry {
    key: KeptKey,
    commands: Vec<SceneCommand>,
    handle: LayerHandle,
    extent: (u32, u32),
    used: u64,
}

#[derive(Default)]
pub(crate) struct KeptLayers {
    entries: Vec<Entry>,
    frame: u64,
}

impl KeptLayers {
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        let (frame, stale) = (self.frame, CacheLimits::DEFAULT.kept_layer_stale_frames);
        self.entries.retain(|entry| entry.used + stale >= frame);
    }

    fn make_room(&mut self, pixels: u64) -> bool {
        crate::renderer::cache_limits::make_room(
            &mut self.entries,
            self.frame,
            (pixels, CacheLimits::DEFAULT.kept_layer_pixels),
            |entry| entry.used,
            |entry| u64::from(entry.handle.width) * u64::from(entry.handle.height),
        )
    }
}

pub(super) fn restore(
    ctx: &mut WalkCtx<'_>,
    key: &KeptKey,
    commands: &[SceneCommand],
) -> Option<(LayerHandle, (u32, u32))> {
    let frame = ctx.kept.frame;
    let entry = ctx
        .kept
        .entries
        .iter_mut()
        .find(|entry| entry.key == *key && entry.commands == commands)?;
    entry.used = frame;
    let (width, height) = (entry.handle.width, entry.handle.height);
    let extent = entry.extent;
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let handle = pool.acquire(ctx.device, width, height, LAYER_FORMAT).ok()?;
    let source = entry.handle.view.texture();
    ctx.encoder
        .copy_region(source, handle.view.texture(), (0, 0, width, height));
    Some((handle, extent))
}

pub(super) fn keep(
    ctx: &mut WalkCtx<'_>,
    (key, commands): (KeptKey, &[SceneCommand]),
    view: &wgpu::TextureView,
    extent: (u32, u32),
) -> Result<(), Error> {
    if commands.len() < CacheLimits::DEFAULT.layer_min_commands {
        return Ok(());
    }
    let source = view.texture();
    let (width, height) = (source.width(), source.height());
    if !ctx.kept.make_room(u64::from(width) * u64::from(height)) {
        return Ok(());
    }
    let pool = ctx.layer_pool.get_or_insert_with(Default::default);
    let handle = pool.acquire(ctx.device, width, height, source.format())?;
    ctx.encoder
        .copy_region(source, handle.view.texture(), (0, 0, width, height));
    let used = ctx.kept.frame;
    ctx.kept.entries.push(Entry {
        key,
        commands: commands.to_vec(),
        handle,
        extent,
        used,
    });
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
