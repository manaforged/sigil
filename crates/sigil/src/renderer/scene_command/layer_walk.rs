use crate::hal::{Point, Rect};

use crate::renderer::blur_paint::{BlurPipeline, PassCtx, PassSrc};
use crate::renderer::error::RendererError;
use crate::renderer::frame_upload::UploadCapacities;
use crate::renderer::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::scene_command::walk::{Walk, WalkCtx, translation};
use crate::renderer::scene_command::{CommandScene, LayerEffect, LayerScope, SceneCommand};
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::sprite_paint::SpritePipeline;

#[path = "layer_walk/geometry.rs"]
mod geometry;
use geometry::{local_rect, physical_extent};

enum Node<'a> {
    Command(&'a SceneCommand),
    Layer {
        scope: LayerScope,
        children: Vec<Self>,
    },
}

#[derive(Clone, Copy)]
struct Target<'a> {
    view: &'a wgpu::TextureView,
    bounds: Rect,
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

fn build_tree(commands: &[SceneCommand]) -> Vec<Node<'_>> {
    let mut index = 0;
    build_until_pop(commands, &mut index)
}

fn build_until_pop<'a>(commands: &'a [SceneCommand], index: &mut usize) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    while let Some(command) = commands.get(*index) {
        *index += 1;
        match command {
            SceneCommand::PushLayer(scope) => {
                let children = build_until_pop(commands, index);
                nodes.push(Node::Layer {
                    scope: *scope,
                    children,
                });
            }
            SceneCommand::PopLayer => break,
            other => nodes.push(Node::Command(other)),
        }
    }
    nodes
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
            Node::Command(_) => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter();
    let mut batch: Vec<&SceneCommand> = Vec::new();
    for node in nodes {
        match node {
            Node::Command(command) => batch.push(command),
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
        reborrow_ctx(ctx, target.view, (clip.width, clip.height)),
        capacity,
    );
    walk.push_clip(Rect::new(Point::new(0.0, 0.0), clip.width, clip.height));
    walk.push_transform(translation(-clip.origin.x, -clip.origin.y));
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
        },
    )?;
    let blurred = match scope.effect {
        LayerEffect::LayerBlur { sigma } if sigma > 0.0 => {
            blurred_view(ctx, &handle, sigma * ctx.scale)?
        }
        LayerEffect::Identity | LayerEffect::LayerBlur { .. } => None,
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
    let pipeline = layer_pipeline_for(ctx.layer_pipelines, ctx.device, ctx.format);
    pipeline.composite(
        PaintCtx {
            device: ctx.device,
            queue: ctx.queue,
            encoder: &mut *ctx.encoder,
            view: target.view,
            viewport: (target.bounds.width, target.bounds.height),
            clip: None,
            scale: ctx.scale,
        },
        CompositeRequest {
            layer_view: &layer.view,
            dest: local_rect(layer.scope.clip, target.bounds.origin),
            texture_size: (layer.handle.width, layer.handle.height),
            source_extent: layer.extent,
            opacity: layer.scope.opacity,
            clip_radii: layer.scope.clip_radii,
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

fn layer_pipeline_for<'a>(
    pipelines: &'a mut Vec<LayerCompositePipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut LayerCompositePipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(LayerCompositePipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("layer pipeline index was just constructed from the vec")
}

fn blur_pipeline_for<'a>(
    pipelines: &'a mut Vec<BlurPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut BlurPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(BlurPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("blur pipeline index was just constructed from the vec")
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

fn layer_depth(nodes: &[Node<'_>]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Command(_) => 0,
            Node::Layer { children, .. } => 1 + layer_depth(children),
        })
        .max()
        .unwrap_or(0)
}
