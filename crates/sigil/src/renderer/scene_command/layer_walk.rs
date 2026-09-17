use crate::hal::{Point, Rect};

use crate::renderer::blur_paint::{PassCtx, PassSrc};
use crate::renderer::error::RendererError;
use crate::renderer::frame_upload::UploadCapacities;
use crate::renderer::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::scene_command::walk::{
    IDENTITY, Walk, WalkCtx, compose, transform_rect, translation,
};
use crate::renderer::scene_command::{
    CommandScene, LayerEffect, LayerScope, SceneCommand, Transform,
};
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::sprite_paint::SpritePipeline;

#[path = "layer_walk/geometry.rs"]
mod geometry;
use geometry::{local_rect, physical_extent};
#[path = "layer_walk/pipelines.rs"]
mod pipelines;
use pipelines::blur_pipeline_for;
#[path = "layer_walk/tree.rs"]
mod tree;
use tree::{Node, build_tree, layer_depth};

#[derive(Clone, Copy)]
struct Target<'a> {
    view: &'a wgpu::TextureView,
    bounds: Rect,
    clip: Option<Rect>,
    transform: Transform,
    opacity: f32,
}

impl Target<'_> {
    fn scoped(mut self, command: &SceneCommand) -> Option<Self> {
        match command {
            SceneCommand::PushClip(rect) => {
                let rect = transform_rect(self.transform, *rect);
                self.clip = Some(
                    self.clip
                        .map_or(Some(rect), |outer| outer.intersection(rect))?,
                );
            }
            SceneCommand::PushTransform(transform) => {
                self.transform = compose(self.transform, *transform);
            }
            SceneCommand::SaveLayer { opacity } => self.opacity *= opacity.clamp(0.0, 1.0),
            _ => unreachable!("scope nodes contain only scope commands"),
        }
        Some(self)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "GPU texture dimensions fit exactly in f32"
    )]
    fn viewport(self, scale: f32) -> (f32, f32) {
        (
            self.view.texture().width() as f32 / scale,
            self.view.texture().height() as f32 / scale,
        )
    }
}

struct Layer {
    scope: LayerScope,
    handle: LayerHandle,
    view: wgpu::TextureView,
    extent: (u32, u32),
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
        clip: None,
        transform: IDENTITY,
        opacity: 1.0,
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
        .prepare(ctx.device, ctx.queue, commands);
}

fn render_nodes(
    ctx: &mut WalkCtx<'_>,
    mut nodes: &[Node<'_>],
    target: Target<'_>,
) -> Result<(), RendererError> {
    while !nodes.is_empty() {
        let end = batch_len(nodes, target.bounds, ctx.scale, ctx.format);
        let (batch, remaining) = nodes.split_at(end);
        render_batch(ctx, batch, target)?;
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
            let Node::Layer { scope, .. } = node else {
                return false;
            };
            let next = LayerPool::bytes(physical_extent(scope.clip, scale), format);
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
    target: Target<'_>,
) -> Result<(), RendererError> {
    let mut layers = nodes
        .iter()
        .filter_map(|node| match node {
            Node::Layer { scope, children } => Some(render_layer(ctx, *scope, children)),
            Node::Command(_) | Node::Scope { .. } => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter();
    let mut batch: Vec<&SceneCommand> = Vec::new();
    for node in nodes {
        match node {
            Node::Command(command) => batch.push(command),
            Node::Scope { command, children } => {
                flush_command_batch(ctx, &mut batch, target);
                if let Some(scoped) = target.scoped(command) {
                    render_nodes(ctx, children, scoped)?;
                }
            }
            Node::Layer { .. } => {
                flush_command_batch(ctx, &mut batch, target);
                let layer = layers.next().expect("layer prepared for each layer node");
                composite_layer(ctx, target, &layer);
                ctx.layer_pool
                    .as_mut()
                    .expect("layer pool acquired the texture")
                    .release(layer.handle);
            }
        }
    }
    flush_command_batch(ctx, &mut batch, target);
    Ok(())
}

fn flush_command_batch(ctx: &mut WalkCtx<'_>, batch: &mut Vec<&SceneCommand>, target: Target<'_>) {
    if batch.is_empty() {
        return;
    }
    let capacity = UploadCapacities::from_commands(batch.iter().copied());
    let clip = target.bounds;
    let mut walk = Walk::new(
        reborrow_ctx(ctx, target.view, target.viewport(ctx.scale)),
        capacity,
    );
    walk.push_clip(local_rect(target.clip.unwrap_or(clip), clip.origin));
    walk.push_transform(translation(-clip.origin.x, -clip.origin.y));
    walk.push_transform(target.transform);
    walk.handle(&SceneCommand::SaveLayer {
        opacity: target.opacity,
    });
    for command in batch.drain(..) {
        walk.handle(command);
    }
    walk.finish();
}

fn render_layer(
    ctx: &mut WalkCtx<'_>,
    scope: LayerScope,
    children: &[Node<'_>],
) -> Result<Layer, RendererError> {
    debug_assert!(
        layer_depth(children) < 8,
        "SceneCommand layer nesting exceeds maximum depth 8"
    );
    let content_extent = physical_extent(scope.clip, ctx.scale);
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, content_extent.0, content_extent.1, ctx.format)
        .map_err(RendererError::Layer)?;
    ctx.stats
        .record_offscreen(content_extent.0, content_extent.1);
    clear_view(ctx.encoder, &handle.view);
    render_nodes(
        ctx,
        children,
        Target {
            view: &handle.view,
            bounds: scope.clip,
            clip: Some(scope.clip),
            transform: IDENTITY,
            opacity: 1.0,
        },
    )?;
    let blurred = match scope.effect {
        LayerEffect::LayerBlur { sigma } if sigma > 0.0 => {
            blurred_view(ctx, &handle, sigma * ctx.scale)?
        }
        LayerEffect::Identity | LayerEffect::Transform(_) | LayerEffect::LayerBlur { .. } => None,
    };
    Ok(Layer {
        scope,
        view: blurred.unwrap_or_else(|| handle.view.clone()),
        handle,
        extent: content_extent,
    })
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
    let blur = blur_pipeline_for(ctx.blur_pipelines, ctx.device, ctx.format);
    let src = PassSrc {
        view: &handle.view,
        w: handle.width,
        h: handle.height,
    };
    blur.apply(&mut pass, pool, src, sigma)
}

fn composite_layer(ctx: &mut WalkCtx<'_>, target: Target<'_>, layer: &Layer) {
    let pipeline = LayerCompositePipeline::for_format(ctx.layer_pipelines, ctx.device, ctx.format);
    pipeline.composite(
        PaintCtx {
            device: ctx.device,
            queue: ctx.queue,
            encoder: &mut *ctx.encoder,
            view: target.view,
            viewport: target.viewport(ctx.scale),
            clip: target
                .clip
                .map(|clip| local_rect(clip, target.bounds.origin)),
            scale: ctx.scale,
        },
        CompositeRequest {
            layer_view: &layer.view,
            dest: local_rect(layer.scope.clip, target.bounds.origin),
            texture_size: (layer.handle.width, layer.handle.height),
            source_extent: layer.extent,
            opacity: layer.scope.opacity * target.opacity,
            clip_radii: layer.scope.clip_radii,
            transform: compose(
                translation(-target.bounds.origin.x, -target.bounds.origin.y),
                compose(
                    target.transform,
                    compose(
                        match layer.scope.effect {
                            LayerEffect::Transform(transform) => transform,
                            _ => IDENTITY,
                        },
                        translation(target.bounds.origin.x, target.bounds.origin.y),
                    ),
                ),
            ),
        },
    );
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
        stats: &mut *ctx.stats,
    }
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
