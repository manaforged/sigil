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
use crate::frame_upload::UploadCapacities;
use crate::layer_composite::{CompositeRequest, LayerCompositePipeline};
use crate::scene_command::walk::{Walk, WalkCtx, translation};
use crate::scene_command::{CommandScene, LayerEffect, LayerScope, SceneCommand};
use crate::scene_paint::PaintCtx;

#[path = "layer_walk/geometry.rs"]
mod geometry;
use geometry::{local_rect, physical_extent};

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
                render_layer(ctx, *scope, children, view, viewport, Point::new(0.0, 0.0))?;
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
    let capacity = UploadCapacities::from_commands(batch.iter().copied());
    let mut walk = Walk::new(reborrow_ctx(ctx, view, viewport), capacity);
    for command in batch.drain(..) {
        walk.handle(command)?;
    }
    walk.finish();
    Ok(())
}

fn render_layer(
    ctx: &mut WalkCtx<'_>,
    scope: LayerScope,
    children: &[Node],
    parent_view: &wgpu::TextureView,
    parent_viewport: (f32, f32),
    parent_origin: Point,
) -> Result<(), RendererError> {
    debug_assert!(
        layer_depth(children) < 8,
        "SceneCommand layer nesting exceeds maximum depth 8"
    );
    let content_extent = physical_extent(scope.clip, ctx.scale);
    let Some(handle) = ctx.layer_pool.get_or_insert_with(Default::default).acquire(
        ctx.device,
        content_extent.0,
        content_extent.1,
        ctx.format,
    ) else {
        return Ok(());
    };
    ctx.stats
        .record_offscreen(content_extent.0, content_extent.1);
    clear_view(ctx.encoder, &handle.view);
    render_layer_children(ctx, children, scope.clip, &handle.view)?;
    apply_effect_and_composite(
        ctx,
        parent_view,
        parent_viewport,
        parent_origin,
        scope,
        &handle,
        content_extent,
    );
    // The composite pass that samples `handle` only runs when the owner submits
    // the encoder, so the texture must outlive this function — retain it.
    ctx.retained.push(handle);
    Ok(())
}

fn apply_effect_and_composite(
    ctx: &mut WalkCtx<'_>,
    parent_view: &wgpu::TextureView,
    parent_viewport: (f32, f32),
    parent_origin: Point,
    scope: LayerScope,
    handle: &crate::layer_pool::LayerHandle,
    content_extent: (u32, u32),
) {
    let blurred = match scope.effect {
        LayerEffect::LayerBlur { sigma } if sigma > 0.0 => {
            blurred_view(ctx, handle, sigma * ctx.scale)
        }
        LayerEffect::Identity | LayerEffect::LayerBlur { .. } => None,
    };
    let view = blurred.as_ref().unwrap_or(&handle.view);
    composite_layer(
        ctx,
        parent_view,
        parent_viewport,
        parent_origin,
        scope,
        view,
        handle_size(handle),
        content_extent,
    );
}

fn blurred_view(
    ctx: &mut WalkCtx<'_>,
    handle: &crate::layer_pool::LayerHandle,
    sigma: f32,
) -> Option<wgpu::TextureView> {
    let mut pass = PassCtx {
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
    blur.apply(&mut pass, pool, src, sigma)
}

fn render_layer_children(
    ctx: &mut WalkCtx<'_>,
    children: &[Node],
    clip: Rect,
    view: &wgpu::TextureView,
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
                flush_layer_batch(ctx, &mut batch, clip, view)?;
                render_layer(
                    ctx,
                    *scope,
                    nested,
                    view,
                    (clip.width, clip.height),
                    clip.origin,
                )?;
            }
        }
    }
    flush_layer_batch(ctx, &mut batch, clip, view)
}

fn flush_layer_batch(
    ctx: &mut WalkCtx<'_>,
    batch: &mut Vec<&SceneCommand>,
    clip: Rect,
    view: &wgpu::TextureView,
) -> Result<(), RendererError> {
    if batch.is_empty() {
        return Ok(());
    }
    let capacity = UploadCapacities::from_commands(batch.iter().copied());
    let mut walk = Walk::new(reborrow_ctx(ctx, view, (clip.width, clip.height)), capacity);
    walk.push_clip(Rect::new(Point::new(0.0, 0.0), clip.width, clip.height));
    walk.push_transform(translation(-clip.origin.x, -clip.origin.y));
    for command in batch.drain(..) {
        walk.handle(command)?;
    }
    walk.finish();
    Ok(())
}

fn composite_layer(
    ctx: &mut WalkCtx<'_>,
    parent_view: &wgpu::TextureView,
    parent_viewport: (f32, f32),
    parent_origin: Point,
    scope: LayerScope,
    layer_view: &wgpu::TextureView,
    texture_size: (u32, u32),
    content_extent: (u32, u32),
) {
    let pipeline = layer_pipeline_for(ctx.layer_pipelines, ctx.device, ctx.format);
    pipeline.composite(
        PaintCtx {
            device: ctx.device,
            queue: ctx.queue,
            encoder: &mut *ctx.encoder,
            view: parent_view,
            viewport: parent_viewport,
            clip: None,
            scale: ctx.scale,
        },
        CompositeRequest {
            layer_view,
            dest: local_rect(scope.clip, parent_origin),
            texture_size,
            source_extent: content_extent,
            opacity: scope.opacity,
            clip_radii: scope.clip_radii,
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

fn handle_size(handle: &crate::layer_pool::LayerHandle) -> (u32, u32) {
    (handle.width, handle.height)
}
