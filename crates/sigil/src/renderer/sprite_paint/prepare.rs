use crate::geometry::Rect;
use crate::renderer::scene_command::state::{MAX_LAYER_EXTENT, SceneState};
use crate::renderer::scene_command::{LayerEffect, SceneCommand, Transform};

use super::SpritePipeline;

pub(crate) fn raster_scale(transform: Transform, scale: f32, dest: Rect) -> f32 {
    let [[xx, xy, _], [yx, yy, _]] = transform;
    let reach = xx.hypot(yx).max(xy.hypot(yy));
    let reach = if reach.is_normal() { reach } else { 1.0 };
    let wanted = scale * reach;
    let extent = dest.width.abs().max(dest.height.abs());
    if extent.is_normal() && extent * wanted > MAX_LAYER_EXTENT {
        (MAX_LAYER_EXTENT / extent).max(scale.min(wanted))
    } else {
        wanted
    }
}

impl SpritePipeline {
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        commands: &[SceneCommand],
        scale: f32,
    ) {
        let mut state = SceneState::default();
        let mut layers: Vec<bool> = Vec::new();
        for command in commands {
            match command {
                SceneCommand::Image(draw) => {
                    let at = raster_scale(state.current_transform(), scale, draw.destination);
                    self.register_image(device, queue, &draw.source_at(at).0);
                }
                SceneCommand::Sprite(draw) => {
                    self.resolve_atlas(device, queue, draw.source.atlas);
                }
                SceneCommand::PushLayer(scope) => {
                    let pushed = matches!(scope.effect, LayerEffect::Transform(_));
                    if let LayerEffect::Transform(effect) = scope.effect {
                        state.push_transform(effect);
                    }
                    layers.push(pushed);
                }
                SceneCommand::PopLayer => {
                    if layers.pop() == Some(true) {
                        state.pop_transform();
                    }
                }
                other => {
                    let _ = state.apply(other);
                }
            }
        }
    }
}
