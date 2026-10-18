use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::{LayerEffect, LayerScope, SceneCommand};
use crate::scene::geometry::{compose, intersect_rect};

use super::{MAX_BLUR_REACH, SceneState, invert, snap_out};

impl SceneState {
    pub(crate) fn local_view(&self, area: Rect) -> Option<Rect> {
        let visible = self.clip_rect(area)?;
        let inverse = invert(self.current_transform())?;
        Some(transform_rect(inverse, visible))
    }

    pub(crate) fn layer_bounds(&self, scope: &LayerScope, area: Rect) -> Option<Rect> {
        if self.clipped_out() {
            return None;
        }
        let visible = self
            .current_clip()
            .map_or(Some(area), |clip| intersect_rect(clip, area))?;
        match scope.effect {
            LayerEffect::Transform(effect) => {
                let inverse = invert(compose(self.current_transform(), effect))?;
                intersect_rect(scope.clip, transform_rect(inverse, visible))
            }
            LayerEffect::LayerBlur { sigma } => {
                let reach = (sigma.max(0.0) * 3.0).min(MAX_BLUR_REACH);
                self.target_clip(scope, visible.expand(reach))
            }
            LayerEffect::Identity => self.target_clip(scope, visible),
        }
    }

    pub(crate) fn layer_region(bounds: Rect, area: Rect, scale: f32) -> Rect {
        snap_out(bounds, area.origin, scale)
    }

    fn target_clip(&self, scope: &LayerScope, visible: Rect) -> Option<Rect> {
        intersect_rect(
            transform_rect(self.current_transform(), scope.clip),
            visible,
        )
    }

    pub(crate) fn group_state(&self, bounds: Rect) -> Self {
        Self::new(
            Some(self.clip_rect(bounds).unwrap_or(bounds)),
            self.current_transform(),
            1.0,
        )
    }

    pub(crate) const fn draws(command: &SceneCommand) -> bool {
        !matches!(
            command,
            SceneCommand::PushClip(_)
                | SceneCommand::PopClip
                | SceneCommand::PushTransform(_)
                | SceneCommand::PopTransform
                | SceneCommand::SaveLayer { .. }
                | SceneCommand::RestoreLayer
                | SceneCommand::PopLayer
        )
    }

    pub(crate) fn composites_group(children: &[SceneCommand]) -> bool {
        children
            .iter()
            .filter(|command| Self::draws(command))
            .nth(1)
            .is_some()
    }

    pub(crate) fn has_composited_group(commands: &[SceneCommand]) -> bool {
        commands.iter().enumerate().any(|(index, command)| {
            matches!(command, SceneCommand::SaveLayer { .. })
                && Self::composites_group(
                    commands
                        .get(index + 1..Self::group_end(commands, index + 1))
                        .unwrap_or_default(),
                )
        })
    }
}
