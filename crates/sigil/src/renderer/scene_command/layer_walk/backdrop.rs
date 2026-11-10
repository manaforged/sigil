#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel windows are clamped to the texture size before the casts"
)]

use crate::Error;
use crate::geometry::{Point, Rect};
use crate::renderer::layer_composite::{Backdrop, CompositeRequest, LayerCompositePipeline};
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::plan::PlanNode;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{IDENTITY, WalkCtx, transform_rect};
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::command::BlendMode;
use crate::scene::corner_radii::CornerRadii;
use crate::style::Color;

use super::{LayerTarget, local_rect, render_nodes};

pub(super) struct BackdropCopy {
    pub(super) handle: LayerHandle,
    origin: [f32; 2],
}

impl BackdropCopy {
    pub(super) const fn backdrop(&self, mode: BlendMode) -> Backdrop<'_> {
        Backdrop {
            view: &self.handle.view,
            origin: self.origin,
            mode,
        }
    }
}

pub(super) fn copy(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    request: &CompositeRequest<'_>,
) -> Option<BackdropCopy> {
    let texture = target.view.texture();
    if !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
        return None;
    }
    let drawn = transform_rect(request.transform, request.dest);
    let area = match target.state.current_clip() {
        Some(clip) => drawn.intersection(local_rect(clip, target.bounds.origin))?,
        None => drawn,
    };
    let window = pixel_window(area, ctx.scale, (texture.width(), texture.height()))?;
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, window.2, window.3, texture.format())
        .ok()?;
    ctx.encoder
        .copy_region(texture, handle.view.texture(), window);
    Some(BackdropCopy {
        handle,
        origin: [window.0 as f32, window.1 as f32],
    })
}

fn pixel_window(
    area: Rect,
    scale: f32,
    (width, height): (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    let edge = |value: f32, limit: u32| (value * scale).clamp(0.0, limit as f32);
    let left = edge(area.origin.x, width).floor() as u32;
    let top = edge(area.origin.y, height).floor() as u32;
    let right = edge(area.origin.x + area.width, width).ceil() as u32;
    let bottom = edge(area.origin.y + area.height, height).ceil() as u32;
    (right > left && bottom > top).then(|| (left, top, right - left, bottom - top))
}

pub(super) fn needs_isolation(commands: &[SceneCommand], view: &wgpu::TextureView) -> bool {
    !view.texture().usage().contains(wgpu::TextureUsages::COPY_SRC)
        && commands.iter().any(|command| {
            matches!(command, SceneCommand::PushLayer(scope) if scope.blend != BlendMode::Normal)
        })
}

pub(super) fn render_isolated(
    ctx: &mut WalkCtx<'_>,
    nodes: &[PlanNode<'_>],
    target: LayerTarget,
) -> Result<(), Error> {
    let viewport = target.viewport(ctx.scale);
    let LayerTarget { view, bounds, .. } = target;
    let size = view.texture().size();
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, size.width, size.height, ctx.format)?;
    ctx.encoder.clear(&handle.view, Color::TRANSPARENT);
    let inner = LayerTarget {
        view: handle.view.clone(),
        bounds,
        state: SceneState::default(),
    };
    let result = render_nodes(ctx, nodes, inner);
    let whole = Rect::new(Point::new(0.0, 0.0), bounds.width, bounds.height);
    let request = CompositeRequest {
        layer_view: &handle.view,
        dest: whole,
        round: whole,
        texture_size: (handle.width, handle.height),
        source_extent: (size.width, size.height),
        opacity: 1.0,
        clip_radii: CornerRadii::ZERO,
        transform: IDENTITY,
    };
    let pipeline = LayerCompositePipeline::for_format(ctx.layer_pipelines, ctx.device, ctx.format);
    let paint = PaintCtx {
        device: ctx.device,
        queue: ctx.queue,
        encoder: &mut *ctx.encoder,
        view: &view,
        viewport,
        clip: None,
        scale: ctx.scale,
    };
    pipeline.composite(paint, request);
    if let Some(pool) = ctx.layer_pool.as_mut() {
        pool.release(handle);
    }
    result
}
