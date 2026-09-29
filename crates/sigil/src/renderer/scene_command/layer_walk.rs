use crate::geometry::{Point, Rect};

use crate::renderer::error::RendererError;
use crate::renderer::frame_upload::UploadCapacities;
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{Walk, WalkCtx, translation};
use crate::renderer::scene_command::{CommandScene, SceneCommand};
use crate::renderer::sprite_paint::SpritePipeline;

#[path = "layer_walk/geometry.rs"]
mod geometry;
use geometry::{local_rect, physical_extent};
#[path = "layer_walk/pipelines.rs"]
mod pipelines;
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
    prepare: Prepare<'n>,
}

impl<'n> Frame<'n> {
    fn new(ctx: &WalkCtx<'_>, nodes: &'n [Node<'n>], target: Target) -> Self {
        let end = batch_len(nodes, target.bounds, ctx.scale, ctx.format);
        let (batch, rest) = nodes.split_at(end);
        let prepare = Prepare::new(batch, &target);
        Self {
            target,
            batch,
            rest,
            prepare,
        }
    }
}

struct Prepare<'n> {
    nodes: &'n [Node<'n>],
    target: Target,
    layers: Vec<Option<Layer>>,
}

impl<'n> Prepare<'n> {
    fn new(nodes: &'n [Node<'n>], target: &Target) -> Self {
        Self {
            nodes,
            target: target.clone(),
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
                self.top().prepare.layers.push(Some(layer));
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
    let index = ctx
        .sprite_pipelines
        .iter()
        .position(|pipeline| pipeline.format() == ctx.format)
        .unwrap_or_else(|| {
            ctx.sprite_pipelines
                .push(SpritePipeline::new(ctx.device, ctx.format));
            ctx.sprite_pipelines.len() - 1
        });
    ctx.sprite_pipelines
        .get_mut(index)
        .expect("sprite pipeline index was just constructed from the vec")
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
        if let Some(nested) = prepare_layers(ctx, &mut frame.prepare)? {
            return Ok(Some(nested));
        }
        let prepared = std::mem::take(&mut frame.prepare.layers);
        render_batch(ctx, frame.batch, &mut frame.target, prepared);
        if frame.rest.is_empty() {
            return Ok(None);
        }
        let end = batch_len(frame.rest, frame.target.bounds, ctx.scale, ctx.format);
        (frame.batch, frame.rest) = frame.rest.split_at(end);
        frame.prepare = Prepare::new(frame.batch, &frame.target);
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
    prepare: &mut Prepare<'n>,
) -> Result<Option<Nested<'n>>, RendererError> {
    while let Some((node, rest)) = prepare.nodes.split_first() {
        prepare.nodes = rest;
        let (plan, children) = match node {
            Node::Command(_) => continue,
            Node::State(command) => {
                let _ = prepare.target.state.apply(command);
                continue;
            }
            Node::Layer { scope, children } => {
                (layer_plan(*scope, &prepare.target, ctx.scale), children)
            }
            Node::Group {
                opacity,
                commands,
                children,
            } => (
                group_plan(*opacity, commands, &prepare.target, ctx.scale),
                children,
            ),
        };
        let Some(plan) = plan else {
            prepare.layers.push(None);
            continue;
        };
        let (open, target) = open_layer(ctx, plan)?;
        let frame = Frame::new(ctx, children, target);
        return Ok(Some(Nested { open, frame }));
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
