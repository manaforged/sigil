use crate::hal::{Point, Rect};

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
use offscreen::{Layer, composite_layer, render_group, render_layer};

#[derive(Clone)]
struct Target<'a> {
    view: &'a wgpu::TextureView,
    bounds: Rect,
    state: SceneState,
}

impl Target<'_> {
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

pub(crate) fn walk_layer_command_scene(
    mut ctx: WalkCtx<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    prepare_sprites(&mut ctx, &scene.commands);
    let nodes = build_tree(&scene.commands);
    let target = Target {
        view: ctx.view,
        bounds: Rect::new(Point::new(0.0, 0.0), ctx.viewport.0, ctx.viewport.1),
        state: SceneState::default(),
    };
    let result = render_nodes(&mut ctx, &nodes, &target);
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
        .prepare(ctx.device, ctx.queue, commands);
}

fn render_nodes(
    ctx: &mut WalkCtx<'_>,
    mut nodes: &[Node<'_>],
    target: &Target<'_>,
) -> Result<(), RendererError> {
    let mut target = target.clone();
    while !nodes.is_empty() {
        let end = batch_len(nodes, target.bounds, ctx.scale, ctx.format);
        let (batch, remaining) = nodes.split_at(end);
        render_batch(ctx, batch, &mut target)?;
        nodes = remaining;
    }
    Ok(())
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
    target: &mut Target<'_>,
) -> Result<(), RendererError> {
    let mut layers = prepare_layers(ctx, nodes, target.clone())?.into_iter();
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
    Ok(())
}

fn prepare_layers(
    ctx: &mut WalkCtx<'_>,
    nodes: &[Node<'_>],
    mut target: Target<'_>,
) -> Result<Vec<Option<Layer>>, RendererError> {
    let mut layers = Vec::new();
    for node in nodes {
        match node {
            Node::Command(_) => {}
            Node::State(command) => {
                let _ = target.state.apply(command);
            }
            Node::Layer { scope, children } => {
                layers.push(render_layer(ctx, *scope, &target, children)?);
            }
            Node::Group {
                opacity,
                commands,
                children,
            } => layers.push(render_group(ctx, *opacity, &target, commands, children)?),
        }
    }
    Ok(layers)
}

fn flush_command_batch(ctx: &mut WalkCtx<'_>, batch: &mut Vec<&SceneCommand>, target: &Target<'_>) {
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
        reborrow_ctx(ctx, target.view, target.viewport(ctx.scale)),
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
