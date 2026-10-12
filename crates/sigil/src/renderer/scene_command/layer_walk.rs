use crate::geometry::{Point, Rect};

use crate::renderer::error::RendererError;
use crate::renderer::frame_upload::UploadCapacities;
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{Walk, WalkCtx, pipeline_for, translation};
use crate::renderer::scene_command::{CommandScene, SceneCommand};
use crate::renderer::sprite_paint::SpritePipeline;

#[path = "layer_walk/tree.rs"]
mod tree;
use tree::{Node, build_tree};
#[path = "layer_walk/offscreen.rs"]
mod offscreen;
use offscreen::{
    Layer, OpenLayer, close_layer, composite_layer, group_plan, layer_plan, open_layer,
};

#[derive(Clone)]
struct Target {
    view: wgpu::TextureView,
    bounds: Rect,
    state: SceneState,
}

impl Target {
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

struct Frame<'n> {
    target: Target,
    batch: &'n [Node<'n>],
    rest: &'n [Node<'n>],
    scan: &'n [Node<'n>],
    scan_target: Target,
    layers: Vec<Option<Layer>>,
}

impl<'n> Frame<'n> {
    fn new(ctx: &WalkCtx<'_>, nodes: &'n [Node<'n>], target: Target) -> Self {
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
    open: OpenLayer,
    frame: Frame<'n>,
}

struct FrameStack<'n> {
    root: Frame<'n>,
    nested: Vec<Nested<'n>>,
}

impl<'n> FrameStack<'n> {
    fn top(&mut self) -> &mut Frame<'n> {
        self.nested
            .last_mut()
            .map_or(&mut self.root, |nested| &mut nested.frame)
    }

    fn run(mut self, ctx: &mut WalkCtx<'_>) -> Result<(), RendererError> {
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
) -> Result<(), RendererError> {
    prepare_sprites(&mut ctx, &scene.commands);
    let nodes = build_tree(&scene.commands);
    let target = Target {
        view: ctx.view.clone(),
        bounds: Rect::new(Point::new(0.0, 0.0), ctx.viewport.0, ctx.viewport.1),
        state: SceneState::default(),
    };
    let result = render_nodes(&mut ctx, &nodes, target);
    if let Some(pool) = ctx.layer_pool.as_ref() {
        for pipeline in &mut *ctx.layer_pipelines {
            pipeline.retain(pool);
        }
        pool.record_profile();
    }
    result
}

fn prepare_sprites(ctx: &mut WalkCtx<'_>, commands: &[SceneCommand]) {
    if !commands
        .iter()
        .any(|command| matches!(command, SceneCommand::Image(_) | SceneCommand::Sprite(_)))
    {
        return;
    }
    pipeline_for(
        ctx.sprite_pipelines,
        ctx.device,
        ctx.format,
        (SpritePipeline::format, SpritePipeline::new),
    )
    .prepare(ctx.device, ctx.queue, commands, ctx.scale);
}

fn render_nodes(
    ctx: &mut WalkCtx<'_>,
    nodes: &[Node<'_>],
    target: Target,
) -> Result<(), RendererError> {
    let format = ctx.format;
    let stack = FrameStack {
        root: Frame::new(ctx, nodes, target),
        nested: Vec::new(),
    };
    let result = stack.run(ctx);
    ctx.format = format;
    result
}

fn advance<'n>(
    ctx: &mut WalkCtx<'_>,
    frame: &mut Frame<'n>,
) -> Result<Option<Nested<'n>>, RendererError> {
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

fn batch_len(nodes: &[Node<'_>], bounds: Rect, scale: f32, format: wgpu::TextureFormat) -> usize {
    let budget = LayerPool::bytes(physical_extent(bounds, scale), format);
    let mut bytes = 0_u64;
    nodes
        .iter()
        .position(|node| {
            let region = match node {
                Node::Layer { scope, .. } => scope.clip,
                Node::Group { .. } => bounds,
                Node::Command(_) | Node::State(_) => return false,
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
    nodes: &[Node<'_>],
    target: &mut Target,
    prepared: Vec<Option<Layer>>,
) {
    let mut layers = prepared.into_iter();
    let mut batch: Vec<&SceneCommand> = Vec::new();
    for node in nodes {
        match node {
            Node::Command(command) => batch.push(command),
            Node::State(command) => {
                flush_command_batch(ctx, &mut batch, target);
                let _ = target.state.apply(command);
            }
            Node::Layer { .. } | Node::Group { .. } => {
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
    frame: &mut Frame<'n>,
) -> Result<Option<Nested<'n>>, RendererError> {
    while let Some((node, rest)) = frame.scan.split_first() {
        frame.scan = rest;
        let (plan, children) = match node {
            Node::Command(_) => continue,
            Node::State(command) => {
                let _ = frame.scan_target.state.apply(command);
                continue;
            }
            Node::Layer { scope, children } => {
                (layer_plan(*scope, &frame.scan_target, ctx.scale), children)
            }
            Node::Group {
                opacity,
                commands,
                children,
            } => (
                group_plan(*opacity, commands, &frame.scan_target, ctx.scale),
                children,
            ),
        };
        let Some(plan) = plan else {
            frame.layers.push(None);
            continue;
        };
        let (open, target) = open_layer(ctx, plan)?;
        let child = Frame::new(ctx, children, target);
        return Ok(Some(Nested { open, frame: child }));
    }
    Ok(None)
}

fn flush_command_batch(ctx: &mut WalkCtx<'_>, batch: &mut Vec<&SceneCommand>, target: &Target) {
    if batch.is_empty() {
        return;
    }
    if target.state.clipped_out() {
        batch.clear();
        return;
    }
    let capacity = UploadCapacities::from_commands(batch.iter().copied());
    let clip = target.bounds;
    let mut walk = Walk::new(
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
        raster_pipelines: &mut *ctx.raster_pipelines,
        sprite_pipelines: &mut *ctx.sprite_pipelines,
        decoration_pipelines: &mut *ctx.decoration_pipelines,
        layer_pipelines: &mut *ctx.layer_pipelines,
        blur_pipelines: &mut *ctx.blur_pipelines,
        layer_pool: &mut *ctx.layer_pool,
        pending: &mut *ctx.pending,
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
