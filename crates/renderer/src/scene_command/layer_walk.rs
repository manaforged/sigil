#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::missing_const_for_fn,
    clippy::too_many_arguments
)]
//! Layer-aware command-scene walker.

use sigil_hal::{Point, Rect};

use crate::blur_paint::{BlurPipeline, PassCtx};
use crate::error::RendererError;
use crate::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::scene_command::walk::{Walk, WalkCtx, translation};
use crate::scene_command::{CommandScene, LayerEffect, LayerScope, SceneCommand};
use crate::scene_paint::PaintCtx;

enum Node {
    Command(SceneCommand),
    Layer {
        scope: LayerScope,
        children: Vec<Self>,
    },
}

pub(crate) fn walk_layer_command_scene(
    mut ctx: WalkCtx<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    let nodes = build_tree(&scene.commands);
    let view = ctx.view;
    let viewport = ctx.viewport;
    render_nodes(&mut ctx, &nodes, view, viewport)
}

fn build_tree(commands: &[SceneCommand]) -> Vec<Node> {
    let mut index = 0;
    build_until_pop(commands, &mut index)
}

fn build_until_pop(commands: &[SceneCommand], index: &mut usize) -> Vec<Node> {
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
            other => nodes.push(Node::Command(other.clone())),
        }
    }
    nodes
}

fn render_nodes(
    ctx: &mut WalkCtx<'_>,
    nodes: &[Node],
    view: &wgpu::TextureView,
    viewport: (f32, f32),
) -> Result<(), RendererError> {
    // Single Walk for the whole sibling list so PushTransform / PushClip /
    // SaveLayer state mutations persist across consecutive commands in the
    // same layer scope. Nested layers allocate their own Walk against the
    // child layer's offscreen view via `render_layer`.
    let mut batch: Vec<&SceneCommand> = Vec::new();
    for node in nodes {
        match node {
            Node::Command(command) => batch.push(command),
            Node::Layer { scope, children } => {
                flush_command_batch(ctx, &mut batch, view, viewport)?;
                render_layer(ctx, *scope, children, view, viewport)?;
            }
        }
    }
    flush_command_batch(ctx, &mut batch, view, viewport)
}

fn flush_command_batch(
    ctx: &mut WalkCtx<'_>,
    batch: &mut Vec<&SceneCommand>,
    view: &wgpu::TextureView,
    viewport: (f32, f32),
) -> Result<(), RendererError> {
    if batch.is_empty() {
        return Ok(());
    }
    let mut walk = Walk::new(reborrow_ctx(ctx, view, viewport));
    for command in batch.drain(..) {
        walk.handle(command)?;
    }
    Ok(())
}

fn render_layer(
    ctx: &mut WalkCtx<'_>,
    scope: LayerScope,
    children: &[Node],
    parent_view: &wgpu::TextureView,
    parent_viewport: (f32, f32),
) -> Result<(), RendererError> {
    debug_assert!(
        layer_depth(children) < 8,
        "SceneCommand layer nesting exceeds maximum depth 8"
    );
    let width = scope.clip.width.ceil().max(0.0) as u32;
    let height = scope.clip.height.ceil().max(0.0) as u32;
    let Some(handle) = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, width, height, ctx.format)
    else {
        return Ok(());
    };
    clear_view(ctx.encoder, &handle.view);
    render_layer_children(
        ctx,
        children,
        scope.clip,
        &handle.view,
        (handle.width, handle.height),
    )?;
    apply_effect_and_composite(ctx, parent_view, parent_viewport, scope, &handle);
    // The composite pass that samples `handle` only runs when the owner submits
    // the encoder, so the texture must outlive this function — retain it.
    ctx.retained.push(handle);
    Ok(())
}

fn apply_effect_and_composite(
    ctx: &mut WalkCtx<'_>,
    parent_view: &wgpu::TextureView,
    parent_viewport: (f32, f32),
    scope: LayerScope,
    handle: &crate::layer_pool::LayerHandle,
) {
    match scope.effect {
        LayerEffect::Identity => {
            composite_layer(
                ctx,
                parent_view,
                parent_viewport,
                scope,
                &handle.view,
                handle_size(handle),
            );
        }
        LayerEffect::LayerBlur { sigma } if sigma > 0.0 => {
            // The blurred result is the same logical size as the source layer
            // (the final upsample writes back to the layer dimensions).
            let blurred = {
                let mut pctx = PassCtx {
                    device: ctx.device,
                    encoder: ctx.encoder,
                    retained: ctx.retained,
                };
                let pool = ctx.layer_pool.get_or_insert_with(Default::default);
                let blur = blur_pipeline_for(ctx.blur_pipelines, ctx.device, ctx.format);
                let src = crate::blur_paint::PassSrc {
                    view: &handle.view,
                    w: handle.width,
                    h: handle.height,
                };
                blur.apply(&mut pctx, pool, src, sigma)
            };
            let view = blurred.as_ref().unwrap_or(&handle.view);
            composite_layer(
                ctx,
                parent_view,
                parent_viewport,
                scope,
                view,
                handle_size(handle),
            );
        }
        LayerEffect::LayerBlur { .. } => {
            // sigma == 0: treat as identity
            composite_layer(
                ctx,
                parent_view,
                parent_viewport,
                scope,
                &handle.view,
                handle_size(handle),
            );
        }
    }
}

fn render_layer_children(
    ctx: &mut WalkCtx<'_>,
    children: &[Node],
    clip: Rect,
    view: &wgpu::TextureView,
    texture_size: (u32, u32),
) -> Result<(), RendererError> {
    // Mirror render_nodes: batch sibling commands so a single Walk carries
    // state mutations across them. The layer's clip + translation are pushed
    // once per batch; commands inside the batch see the same stack.
    let mut batch: Vec<&SceneCommand> = Vec::new();
    for node in children {
        match node {
            Node::Command(command) => batch.push(command),
            Node::Layer {
                scope,
                children: nested,
            } => {
                flush_layer_batch(ctx, &mut batch, clip, view, texture_size)?;
                render_layer(ctx, *scope, nested, view, viewport(texture_size))?;
            }
        }
    }
    flush_layer_batch(ctx, &mut batch, clip, view, texture_size)
}

fn flush_layer_batch(
    ctx: &mut WalkCtx<'_>,
    batch: &mut Vec<&SceneCommand>,
    clip: Rect,
    view: &wgpu::TextureView,
    texture_size: (u32, u32),
) -> Result<(), RendererError> {
    if batch.is_empty() {
        return Ok(());
    }
    let mut walk = Walk::new(reborrow_ctx(ctx, view, viewport(texture_size)));
    walk.push_clip(Rect::new(Point::new(0.0, 0.0), clip.width, clip.height));
    walk.push_transform(translation(-clip.origin.x, -clip.origin.y));
    for command in batch.drain(..) {
        walk.handle(command)?;
    }
    Ok(())
}

fn composite_layer(
    ctx: &mut WalkCtx<'_>,
    parent_view: &wgpu::TextureView,
    parent_viewport: (f32, f32),
    scope: LayerScope,
    layer_view: &wgpu::TextureView,
    texture_size: (u32, u32),
) {
    let pipeline = layer_pipeline_for(ctx.layer_pipelines, ctx.device, ctx.format);
    pipeline.composite(
        PaintCtx {
            device: ctx.device,
            queue: ctx.queue,
            encoder: &mut *ctx.encoder,
            view: parent_view,
            viewport: parent_viewport,
        },
        CompositeRequest {
            layer_view,
            dest: scope.clip,
            texture_size,
            opacity: scope.opacity,
        },
    );
}

fn reborrow_ctx<'a>(
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

fn clear_view(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
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

fn layer_depth(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Command(_) => 0,
            Node::Layer { children, .. } => 1 + layer_depth(children),
        })
        .max()
        .unwrap_or(0)
}

const fn viewport(texture_size: (u32, u32)) -> (f32, f32) {
    (texture_size.0 as f32, texture_size.1 as f32)
}

fn handle_size(handle: &crate::layer_pool::LayerHandle) -> (u32, u32) {
    (handle.width, handle.height)
}
