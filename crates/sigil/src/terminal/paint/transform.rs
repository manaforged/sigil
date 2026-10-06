use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::{LayerEffect, Transform};

const IDENTITY: Transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

#[derive(Default)]
pub(super) struct Space {
    transforms: Vec<Transform>,
}

impl Space {
    pub(super) fn push(&mut self, transform: Transform) {
        self.transforms.push(transform);
    }

    pub(super) fn push_effect(&mut self, effect: LayerEffect) {
        match effect {
            LayerEffect::Transform(transform) => self.push(transform),
            LayerEffect::Identity | LayerEffect::LayerBlur { .. } => self.push(IDENTITY),
        }
    }

    pub(super) fn pop(&mut self) {
        self.transforms.pop();
    }

    #[must_use]
    pub(super) fn map(&self, rect: Rect) -> Rect {
        self.transforms
            .iter()
            .rev()
            .fold(rect, |rect, transform| transform_rect(*transform, rect))
    }
}
