use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::{LayerEffect, LayerScope, Transform, Transform3D};
use crate::scene::geometry::intersect_rect;
use crate::scene::state::{MAX_LAYER_EXTENT, SceneState};

#[derive(Clone, Copy)]
pub(crate) struct ProjectedFrame {
    pub(crate) bounds: Rect,
    pub(crate) content: Transform,
    pub(crate) composite: Transform3D,
}

impl ProjectedFrame {
    pub(crate) fn new(
        state: &SceneState,
        scope: &LayerScope,
        area: Rect,
        scale: f32,
    ) -> Option<Self> {
        let LayerEffect::Projected {
            transform,
            hide_backface,
        } = scope.effect
        else {
            return None;
        };
        if state.clipped_out() {
            return None;
        }
        let full =
            Transform3D::from(state.current_transform()).then(transform.visible(hide_backface)?);
        let visible = state
            .current_clip()
            .map_or(Some(area), |clip| intersect_rect(clip, area))?;
        if !full.bounds(scope.clip)?.intersects(visible) {
            return None;
        }
        let fit = resolution(scope.clip, scale);
        let shrink = [[fit, 0.0, 0.0], [0.0, fit, 0.0]];
        let grow = [[1.0 / fit, 0.0, 0.0], [0.0, 1.0 / fit, 0.0]];
        Some(Self {
            bounds: transform_rect(shrink, scope.clip),
            content: shrink,
            composite: full.then(Transform3D::from(grow)),
        })
    }
}

fn resolution(clip: Rect, scale: f32) -> f32 {
    let physical = clip.width.max(clip.height) * scale;
    if physical.is_normal() && physical > MAX_LAYER_EXTENT {
        MAX_LAYER_EXTENT / physical
    } else {
        1.0
    }
}
