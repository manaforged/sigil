use std::sync::Arc;

use crate::scene::command::{Filter, LayerEffect};

#[derive(Clone, Debug)]
pub(crate) struct LayerFilters(Arc<[Filter]>);

impl LayerFilters {
    pub(crate) fn of(effect: &LayerEffect) -> Option<Self> {
        match effect {
            LayerEffect::Filter(filters) if !filters.is_empty() => Some(Self(Arc::clone(filters))),
            _ => None,
        }
    }

    pub(crate) fn steps(&self) -> impl Iterator<Item = &[Filter]> {
        self.0
            .chunk_by(|left, right| left.is_color() && right.is_color())
    }
}
