use std::sync::Arc;

use crate::scene::command::effects::filter_reach;
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

    pub(crate) fn behind(effect: &LayerEffect) -> Option<Self> {
        match effect {
            LayerEffect::Backdrop(filters) if !filters.is_empty() => {
                Some(Self(Arc::clone(filters)))
            }
            _ => None,
        }
    }

    pub(crate) fn reach(&self) -> f32 {
        filter_reach(&self.0)
    }

    pub(crate) fn steps(&self) -> impl Iterator<Item = &[Filter]> {
        self.0
            .chunk_by(|left, right| left.is_color() && right.is_color())
    }
}
