use crate::geometry::{Point, Rect};

use crate::Error;
use crate::renderer::frame_upload::UploadCapacities;
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::plan::{PlanNode, build_tree};
use crate::renderer::quad_paint::AtlasPipeline;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{SceneWalk, WalkCtx, pipeline_for, translation};
use crate::renderer::scene_command::{CommandScene, SceneCommand};

#[path = "layer_walk/backdrop.rs"]
mod backdrop;
#[path = "layer_walk/kept.rs"]
mod kept;
pub(crate) use kept::KeptLayers;
#[path = "layer_walk/mask.rs"]
mod mask;
#[path = "layer_walk/offscreen.rs"]
mod offscreen;
use offscreen::close_layer;
use offscreen::{LayerPlan, OffscreenLayer, OpenLayer, group_plan};
use offscreen::{composite_layer, open_layer};

#[derive(Clone)]
struct LayerTarget {
    view: wgpu::TextureView,
    bounds: Rect,
    state: SceneState,
}

impl LayerTarget {
    #[expect(
        clippy::cast_precision_loss,
        reason = "GPU texture dimensions fit exactly in f32"
    )]
    fn viewport(&self, scale: f32) -> (f32, f32) {
        (
            self.view.texture().width() as f32 / scale,
            self.view.texture().height() as f32 / scale,
        )
    }
}

struct LayerFrame<'n> {
    target: LayerTarget,
    batch: &'n [PlanNode<'n>],
    rest: &'n [PlanNode<'n>],
    scan: &'n [PlanNode<'n>],
    scan_target: LayerTarget,
    layers: Vec<Option<OffscreenLayer<'n>>>,
}

impl<'n> LayerFrame<'n> {
    fn new(ctx: &WalkCtx<'_>, nodes: &'n [PlanNode<'n>], target: LayerTarget) -> Self {
        let end = batch_len(nodes, target.bounds, ctx.scale, ctx.format);
        let (batch, rest) = nodes.split_at(end);
        Self {
            scan_target: target.clone(),
            target,
            batch,
            rest,
            scan: batch,
            layers: Vec::new(),
        }
    }
}

struct Nested<'n> {
    open: OpenLayer<'n>,
    frame: LayerFrame<'n>,
}

struct FrameStack<'n> {
    root: LayerFrame<'n>,
    nested: Vec<Nested<'n>>,
}

impl<'n> FrameStack<'n> {
    fn top(&mut self) -> &mut LayerFrame<'n> {
        self.nested
            .last_mut()
            .map_or(&mut self.root, |nested| &mut nested.frame)
    }

    fn run(mut self, ctx: &mut WalkCtx<'_>) -> Result<(), Error> {
        loop {
            if let Some(nested) = advance(ctx, self.top())? {
                self.nested.push(nested);
            } else if let Some(done) = self.nested.pop() {
                let layer = close_layer(ctx, done.open)?;
                self.top().layers.push(Some(layer));
            } else {
                return Ok(());
            }
        }
    }
}

pub(crate) fn walk_layer_command_scene(
    mut ctx: WalkCtx<'_>,
    scene: &CommandScene,
) -> Result<(), Error> {
    prepare_sprites(&mut ctx, &scene.commands);
    let nodes = build_tree(&scene.commands);
    let target = LayerTarget {
        view: ctx.view.clone(),
        bounds: Rect::new(Point::new(0.0, 0.0), ctx.viewport.0, ctx.viewport.1),
        state: SceneState::default(),
    };
    let result = if backdrop::needs_isolation(&scene.commands, ctx.view) {
        backdrop::render_isolated(&mut ctx, &nodes, target)
    } else {
        render_nodes(&mut ctx, &nodes, target)
    };
    if let Some(pool) = ctx.layer_pool.as_ref() {
        let (washes, vector) = (&*ctx.wash_cache, &*ctx.vector);
        for pipeline in &mut *ctx.layer_pipelines {
            pipeline.retain(|view| {
                pool.contains(view) || washes.contains(view) || vector.contains(view)
            });
        }
        pool.record_profile();
    }
    result
}

fn prepare_sprites(ctx: &mut WalkCtx<'_>, commands: &[SceneCommand]) {
    let sprites = |list: &[SceneCommand]| list.iter().any(SceneCommand::is_image);
    let masks: Vec<&[SceneCommand]> = mask::mask_commands(commands)
        .filter(|list| sprites(list))
        .collect();
    if !sprites(commands) && masks.is_empty() {
        return;
    }
    let atlas = pipeline_for(
        ctx.quad_pipelines,
        ctx.device,
        ctx.format,
        (AtlasPipeline::format, AtlasPipeline::new),
    );
    for list in std::iter::once(commands).chain(masks) {
        atlas.prepare(ctx.device, ctx.queue, list, ctx.scale);
    }
}

fn render_nodes(
    ctx: &mut WalkCtx<'_>,
    nodes: &[PlanNode<'_>],
    target: LayerTarget,
) -> Result<(), Error> {
    let format = ctx.format;
    let stack = FrameStack {
        root: LayerFrame::new(ctx, nodes, target),
        nested: Vec::new(),
    };
    let result = stack.run(ctx);
    ctx.format = format;
    result
}

fn advance<'n>(
    ctx: &mut WalkCtx<'_>,
    frame: &mut LayerFrame<'n>,
) -> Result<Option<Nested<'n>>, Error> {
    loop {
        if let Some(nested) = prepare_layers(ctx, frame)? {
            return Ok(Some(nested));
        }
        let prepared = std::mem::take(&mut frame.layers);
        render_batch(ctx, frame.batch, &mut frame.target, prepared);
        if frame.rest.is_empty() {
            return Ok(None);
        }
        let end = batch_len(frame.rest, frame.target.bounds, ctx.scale, ctx.format);
        (frame.batch, frame.rest) = frame.rest.split_at(end);
        frame.scan = frame.batch;
        frame.scan_target = frame.target.clone();
    }
}

fn batch_len(
    nodes: &[PlanNode<'_>],
    bounds: Rect,
    scale: f32,
    format: wgpu::TextureFormat,
) -> usize {
    let budget = LayerPool::bytes(physical_extent(bounds, scale), format);
    let mut bytes = 0_u64;
    nodes
        .iter()
        .position(|node| {
            let region = match node {
                PlanNode::Layer { scope, .. } => scope.clip,
                PlanNode::Group { .. } => bounds,
                PlanNode::Command(_) | PlanNode::State(_) => return false,
            };
            let next = LayerPool::bytes(physical_extent(region, scale), format);
            if bytes > 0 && bytes.saturating_add(next) > budget {
                return true;
            }
            bytes += next;
            false
        })
        .unwrap_or(nodes.len())
}

fn render_batch(
    ctx: &mut WalkCtx<'_>,
    nodes: &[PlanNode<'_>],
    target: &mut LayerTarget,
    prepared: Vec<Option<OffscreenLayer<'_>>>,
) {
    let mut layers = prepared.into_iter();
    let mut batch: Vec<&SceneCommand> = Vec::new();
    for node in nodes {
        match node {
            PlanNode::Command(command) => batch.push(command),
            PlanNode::State(command) => {
                flush_command_batch(ctx, &mut batch, target);
                let _ = target.state.apply(command);
            }
            PlanNode::Layer { .. } | PlanNode::Group { .. } => {
                flush_command_batch(ctx, &mut batch, target);
                if let Some(Some(layer)) = layers.next() {
                    composite_layer(ctx, target, layer);
                }
            }
        }
    }
    flush_command_batch(ctx, &mut batch, target);
}

fn prepare_layers<'n>(
    ctx: &mut WalkCtx<'_>,
    frame: &mut LayerFrame<'n>,
) -> Result<Option<Nested<'n>>, Error> {
    while let Some((node, rest)) = frame.scan.split_first() {
        frame.scan = rest;
        let opened = match node {
            PlanNode::Command(_) => None,
            PlanNode::State(command) => {
                let _ = frame.scan_target.state.apply(command);
                None
            }
            PlanNode::Layer {
                scope,
                mask,
                children,
                commands,
            } => kept::open_scope(ctx, frame, (scope, *mask), (children, commands))?,
            PlanNode::Group {
                opacity,
                commands,
                children,
            } => {
                let plan = group_plan(*opacity, commands, &frame.scan_target, ctx.scale);
                open_plan(ctx, frame, plan, children)?
            }
        };
        if opened.is_some() {
            return Ok(opened);
        }
    }
    Ok(None)
}

fn open_plan<'n>(
    ctx: &mut WalkCtx<'_>,
    frame: &mut LayerFrame<'n>,
    plan: Option<LayerPlan<'n>>,
    children: &'n [PlanNode<'n>],
) -> Result<Option<Nested<'n>>, Error> {
    let Some(plan) = plan else {
        frame.layers.push(None);
        return Ok(None);
    };
    let (open, target) = open_layer(ctx, plan)?;
    let child = LayerFrame::new(ctx, children, target);
    Ok(Some(Nested { open, frame: child }))
}

fn flush_command_batch(
    ctx: &mut WalkCtx<'_>,
    batch: &mut Vec<&SceneCommand>,
    target: &LayerTarget,
) {
    if batch.is_empty() {
        return;
    }
    if target.state.clipped_out() {
        batch.clear();
        return;
    }
    let capacity = UploadCapacities::from_commands(batch.iter().copied());
    let clip = target.bounds;
    let mut walk = SceneWalk::new(
        reborrow_ctx(ctx, &target.view, target.viewport(ctx.scale)),
        capacity,
    );
    walk.state.push_clip(local_rect(
        target.state.current_clip().unwrap_or(clip),
        clip.origin,
    ));
    walk.state
        .push_transform(translation(-clip.origin.x, -clip.origin.y));
    walk.state.push_transform(target.state.current_transform());
    walk.handle(&SceneCommand::SaveLayer {
        opacity: target.state.current_opacity(),
    });
    for command in batch.drain(..) {
        walk.handle(command);
    }
    walk.finish();
}

const fn reborrow_ctx<'a>(
    ctx: &'a mut WalkCtx<'_>,
    view: &'a wgpu::TextureView,
    viewport: (f32, f32),
) -> WalkCtx<'a> {
    WalkCtx {
        device: ctx.device,
        queue: ctx.queue,
        encoder: &mut *ctx.encoder,
        retained: &mut *ctx.retained,
        view,
        viewport,
        scale: ctx.scale,
        format: ctx.format,
        scene_pipelines: &mut *ctx.scene_pipelines,
        fill_pipelines: &mut *ctx.fill_pipelines,
        shadow_pipelines: &mut *ctx.shadow_pipelines,
        slug_pipelines: &mut *ctx.slug_pipelines,
        quad_pipelines: &mut *ctx.quad_pipelines,
        decoration_pipelines: &mut *ctx.decoration_pipelines,
        layer_pipelines: &mut *ctx.layer_pipelines,
        filter_pipelines: &mut *ctx.filter_pipelines,
        projected_pipelines: &mut *ctx.projected_pipelines,
        shader_programs: &mut *ctx.shader_programs,
        layer_pool: &mut *ctx.layer_pool,
        wash_cache: &mut *ctx.wash_cache,
        pending: &mut *ctx.pending,
        vector: &mut *ctx.vector,
        kept: &mut *ctx.kept,
        stats: &mut *ctx.stats,
    }
}

const fn local_rect(rect: Rect, parent_origin: Point) -> Rect {
    Rect::new(
        Point::new(
            rect.origin.x - parent_origin.x,
            rect.origin.y - parent_origin.y,
        ),
        rect.width,
        rect.height,
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn physical_extent(rect: Rect, scale: f32) -> (u32, u32) {
    let scale = scale.max(0.0);
    (
        (rect.width * scale).ceil().max(0.0) as u32,
        (rect.height * scale).ceil().max(0.0) as u32,
    )
}

#[cfg(test)]
#[path = "layer_walk/geometry_tests.rs"]
mod tests;
