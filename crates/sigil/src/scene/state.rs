use smallvec::{SmallVec, smallvec};

use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::{SceneCommand, Transform};
use crate::scene::geometry::{IDENTITY, compose, intersect_rect};

#[cfg(feature = "scene")]
mod frame;
mod ink;
#[cfg(feature = "scene")]
mod layers;
pub(crate) use ink::{ink_rect, snap_out};
#[cfg(feature = "scene")]
pub(crate) use ink::{invert, shadow_reach};

#[cfg(feature = "scene")]
pub(crate) const MAX_BLUR_REACH: f32 = 1024.0;
#[cfg(feature = "scene")]
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
            SceneCommand::SaveLayer { opacity } => self
                .opacities
                .push(self.current_opacity() * opacity.clamp(0.0, 1.0)),
            SceneCommand::RestoreLayer if self.opacities.len() > 1 => {
                let _ = self.opacities.pop();
            }
            SceneCommand::RestoreLayer => {}
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

    pub(crate) fn current_transform(&self) -> Transform {
        self.transforms.last().copied().unwrap_or(IDENTITY)
    }

    pub(crate) fn current_opacity(&self) -> f32 {
        self.opacities.last().copied().unwrap_or(1.0)
    }

    pub(crate) fn layer_end(commands: &[SceneCommand], start: usize) -> usize {
        matching_end(commands, start, |command| match command {
            SceneCommand::PushLayer(_) => Some(true),
            SceneCommand::PopLayer => Some(false),
            _ => None,
        })
    }

    pub(crate) fn group_end(commands: &[SceneCommand], start: usize) -> usize {
        matching_end(commands, start, |command| match command {
            SceneCommand::SaveLayer { .. } => Some(true),
            SceneCommand::RestoreLayer => Some(false),
            _ => None,
        })
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
}

fn matching_end(
    commands: &[SceneCommand],
    start: usize,
    opens: fn(&SceneCommand) -> Option<bool>,
) -> usize {
    let mut depth = 0_usize;
    for (offset, command) in commands.iter().enumerate().skip(start) {
        match opens(command) {
            Some(true) => depth += 1,
            Some(false) if depth == 0 => return offset,
            Some(false) => depth -= 1,
            None => {}
        }
    }
    commands.len()
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
