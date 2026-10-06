use smallvec::{SmallVec, smallvec};

use crate::geometry::Rect;
use crate::renderer::scene_command::walk::{IDENTITY, compose, intersect_rect, transform_rect};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};

mod frame;
mod ink;
pub(crate) use ink::{ink_rect, invert, shadow_reach, snap_out};

pub(crate) const MAX_BLUR_REACH: f32 = 1024.0;
pub(crate) const MAX_LAYER_EXTENT: f32 = 8191.0;

#[derive(Clone)]
pub(crate) struct SceneState {
    clips: SmallVec<[Rect; 4]>,
    base_clips: usize,
    transforms: SmallVec<[Transform; 4]>,
    opacities: SmallVec<[f32; 4]>,
}

impl Default for SceneState {
    fn default() -> Self {
        Self::new(None, IDENTITY, 1.0)
    }
}

impl SceneState {
    pub(crate) fn new(clip: Option<Rect>, transform: Transform, opacity: f32) -> Self {
        let clips: SmallVec<[Rect; 4]> = clip.into_iter().collect();
        Self {
            base_clips: clips.len(),
            clips,
            transforms: smallvec![transform],
            opacities: smallvec![opacity],
        }
    }

    pub(crate) fn apply(&mut self, command: &SceneCommand) -> bool {
        match command {
            SceneCommand::PushClip(rect) => self.push_clip(*rect),
            SceneCommand::PopClip => self.pop_clip(),
            SceneCommand::PushTransform(transform) => self.push_transform(*transform),
            SceneCommand::PopTransform => self.pop_transform(),
            SceneCommand::SaveLayer { opacity } => self.save_layer(*opacity),
            SceneCommand::RestoreLayer => self.restore_layer(),
            _ => return false,
        }
        true
    }

    pub(crate) fn push_clip(&mut self, rect: Rect) {
        self.clips
            .push(transform_rect(self.current_transform(), rect));
    }

    pub(crate) fn pop_clip(&mut self) {
        if self.clips.len() > self.base_clips {
            let _ = self.clips.pop();
        }
    }

    pub(crate) fn push_transform(&mut self, transform: Transform) {
        self.transforms
            .push(compose(self.current_transform(), transform));
    }

    pub(crate) fn pop_transform(&mut self) {
        if self.transforms.len() > 1 {
            let _ = self.transforms.pop();
        }
    }

    pub(crate) fn save_layer(&mut self, opacity: f32) {
        self.opacities
            .push(self.current_opacity() * opacity.clamp(0.0, 1.0));
    }

    pub(crate) fn restore_layer(&mut self) {
        if self.opacities.len() > 1 {
            let _ = self.opacities.pop();
        }
    }

    pub(crate) fn current_clip(&self) -> Option<Rect> {
        let mut clips = self.clips.iter().copied();
        let first = clips.next()?;
        clips.try_fold(first, intersect_rect)
    }

    pub(crate) fn clipped_out(&self) -> bool {
        !self.clips.is_empty() && self.current_clip().is_none()
    }

    pub(crate) fn clip_rect(&self, rect: Rect) -> Option<Rect> {
        self.clips.iter().copied().try_fold(rect, intersect_rect)
    }

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

    pub(crate) fn current_transform(&self) -> Transform {
        self.transforms.last().copied().unwrap_or(IDENTITY)
    }

    pub(crate) fn current_opacity(&self) -> f32 {
        self.opacities.last().copied().unwrap_or(1.0)
    }

    pub(crate) fn layer_end(commands: &[SceneCommand], start: usize) -> usize {
        let mut depth = 0_usize;
        for (offset, command) in commands.iter().enumerate().skip(start) {
            match command {
                SceneCommand::PushLayer(_) => depth += 1,
                SceneCommand::PopLayer if depth == 0 => return offset,
                SceneCommand::PopLayer => depth -= 1,
                _ => {}
            }
        }
        commands.len()
    }

    pub(crate) fn group_end(commands: &[SceneCommand], start: usize) -> usize {
        let mut depth = 0_usize;
        for (offset, command) in commands.iter().enumerate().skip(start) {
            match command {
                SceneCommand::SaveLayer { .. } => depth += 1,
                SceneCommand::RestoreLayer if depth == 0 => return offset,
                SceneCommand::RestoreLayer => depth -= 1,
                _ => {}
            }
        }
        commands.len()
    }

    pub(crate) fn draw_bounds(
        &self,
        children: &[SceneCommand],
        area: Rect,
        scale: f32,
    ) -> Option<Rect> {
        if self.clipped_out() {
            return None;
        }
        let mut state = Self::new(
            self.current_clip(),
            self.current_transform(),
            self.current_opacity(),
        );
        let mut drawn: Option<Rect> = None;
        let mut index = 0;
        while let Some(command) = children.get(index) {
            index += 1;
            if state.apply(command) {
                continue;
            }
            if matches!(command, SceneCommand::PushLayer(_)) {
                index = Self::layer_end(children, index) + 1;
            }
            let Some(ink) = ink_rect(command, scale) else {
                continue;
            };
            if let Some(rect) = state.clip_rect(transform_rect(state.current_transform(), ink)) {
                drawn = Some(drawn.map_or(rect, |so_far| so_far.union(rect)));
            }
        }
        Some(snap_out(intersect_rect(drawn?, area)?, area.origin, scale))
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

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
