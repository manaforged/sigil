use crate::geometry::Rect;

use crate::renderer::layer_composite::LayerMask;
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::plan::MaskSource;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{WalkCtx, compose, translation};
use crate::renderer::scene_instance::LAYER_FORMAT;
use crate::renderer::vector_pass::{Rendered, Run};
use crate::scene::command::{DrawPath, LayerEffect, Mask, MaskMode};
use crate::style::Color;

use crate::renderer::plan::build_tree;

use super::{LayerTarget, local_rect, render_nodes};

pub(super) enum MaskTexture {
    Path(Rendered),
    Scene {
        handle: LayerHandle,
        origin: (f32, f32),
        mode: MaskMode,
    },
}

impl MaskTexture {
    #[expect(
        clippy::cast_precision_loss,
        reason = "GPU texture coordinates fit exactly in f32"
    )]
    pub(super) const fn layer_mask(&self) -> LayerMask<'_> {
        match self {
            Self::Path(rendered) => LayerMask {
                view: &rendered.view,
                rect: [
                    rendered.origin.0 as f32,
                    rendered.origin.1 as f32,
                    rendered.size.0 as f32,
                    rendered.size.1 as f32,
                ],
                mode: MaskMode::Alpha,
            },
            Self::Scene {
                handle,
                origin,
                mode,
            } => LayerMask {
                view: &handle.view,
                rect: [
                    origin.0,
                    origin.1,
                    handle.width as f32,
                    handle.height as f32,
                ],
                mode: *mode,
            },
        }
    }

    pub(super) fn release(self, ctx: &mut WalkCtx<'_>) {
        if let (Self::Scene { handle, .. }, Some(pool)) = (self, ctx.layer_pool.as_mut()) {
            pool.release(handle);
        }
    }
}

pub(super) fn render(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    source: MaskSource<'_>,
    (dest, extent): (Rect, (u32, u32)),
) -> Option<MaskTexture> {
    match source {
        MaskSource::Path(draw) => render_path(ctx, target, draw, dest).map(MaskTexture::Path),
        MaskSource::Scene(mask) => render_scene(ctx, target, mask, (dest, extent)),
    }
}

fn render_path(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    draw: &DrawPath,
    dest: Rect,
) -> Option<Rendered> {
    let (origin, scale) = (target.bounds.origin, ctx.scale);
    let local = compose(
        translation(-origin.x, -origin.y),
        target.state.current_transform(),
    );
    let extent = target.view.texture().size();
    let side = |value: u32| u16::try_from(value).unwrap_or(u16::MAX);
    let run = Run {
        size: (side(extent.width), side(extent.height)),
        scale,
        visible: local_rect(dest, origin),
        opacity: 1.0,
    };
    let device = compose([[scale, 0.0, 0.0], [0.0, scale, 0.0]], local);
    ctx.vector.queue(draw, device, run);
    ctx.vector.render(ctx.device, ctx.queue, ctx.encoder, false)
}

fn render_scene(
    ctx: &mut WalkCtx<'_>,
    target: &LayerTarget,
    mask: &Mask,
    (dest, extent): (Rect, (u32, u32)),
) -> Option<MaskTexture> {
    let handle = ctx
        .layer_pool
        .get_or_insert_with(Default::default)
        .acquire(ctx.device, extent.0, extent.1, LAYER_FORMAT)
        .ok()?;
    ctx.encoder.clear(&handle.view, Color::TRANSPARENT);
    let layer = LayerTarget {
        view: handle.view.clone(),
        bounds: dest,
        state: SceneState::new(Some(dest), target.state.current_transform(), 1.0),
    };
    let parent_format = std::mem::replace(&mut ctx.format, LAYER_FORMAT);
    let nodes = build_tree(&mask.commands);
    let drawn = render_nodes(ctx, &nodes, layer);
    ctx.format = parent_format;
    if drawn.is_err() {
        if let Some(pool) = ctx.layer_pool.as_mut() {
            pool.release(handle);
        }
        return None;
    }
    let local = local_rect(dest, target.bounds.origin);
    Some(MaskTexture::Scene {
        handle,
        origin: (local.origin.x * ctx.scale, local.origin.y * ctx.scale),
        mode: mask.mode,
    })
}

pub(super) fn mask_commands(commands: &[SceneCommand]) -> impl Iterator<Item = &[SceneCommand]> {
    commands.iter().filter_map(|command| match command {
        SceneCommand::PushLayer(scope) => match &scope.effect {
            LayerEffect::Mask(mask) => Some(&*mask.commands),
            _ => None,
        },
        _ => None,
    })
}
