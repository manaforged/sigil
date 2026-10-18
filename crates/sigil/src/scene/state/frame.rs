use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::{LayerEffect, LayerScope, Transform};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::geometry::compose;

use super::{MAX_LAYER_EXTENT, SceneState};

#[derive(Clone, Copy)]
pub(crate) struct TransformFrame {
    pub(crate) scope: LayerScope,
    pub(crate) bounds: Rect,
    pub(crate) content: Transform,
    pub(crate) composite: Transform,
}

impl SceneState {
    pub(crate) fn transform_frame(
        &self,
        scope: &LayerScope,
        effect: Transform,
        area: Rect,
        scale: f32,
    ) -> Option<TransformFrame> {
        let bounds = self.layer_bounds(scope, area)?;
        let full = compose(self.current_transform(), effect);
        let [[xx, xy, _], [yx, yy, _]] = full;
        let fx = resolution(xx.hypot(yx), bounds.width * scale);
        let fy = resolution(xy.hypot(yy), bounds.height * scale);
        let shrink = [[fx, 0.0, 0.0], [0.0, fy, 0.0]];
        let grow = [[1.0 / fx, 0.0, 0.0], [0.0, 1.0 / fy, 0.0]];
        let radius = fx.min(fy);
        Some(TransformFrame {
            scope: LayerScope {
                clip: transform_rect(shrink, scope.clip),
                clip_radii: CornerRadii(scope.clip_radii.0.map(|r| r * radius)),
                ..*scope
            },
            bounds: transform_rect(shrink, bounds),
            content: shrink,
            composite: compose(full, grow),
        })
    }

    pub(crate) fn inline_transform(scope: &LayerScope) -> Option<Transform> {
        let LayerEffect::Transform(effect) = scope.effect else {
            return None;
        };
        let [[xx, xy, tx], [yx, yy, ty]] = effect;
        let uniform = xx.is_normal()
            && xx > 0.0
            && (xx - yy).abs() <= f32::EPSILON * xx
            && xy.abs() < f32::MIN_POSITIVE
            && yx.abs() < f32::MIN_POSITIVE
            && tx.is_finite()
            && ty.is_finite();
        (uniform && scope.opacity >= 1.0 && scope.clip_radii == CornerRadii::ZERO).then_some(effect)
    }
}

fn resolution(reach: f32, physical: f32) -> f32 {
    let reach = if reach.is_normal() { reach } else { 1.0 };
    if physical.is_normal() && physical * reach > MAX_LAYER_EXTENT {
        MAX_LAYER_EXTENT / physical
    } else {
        reach
    }
}
