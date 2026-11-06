use std::sync::Arc;

use crate::scene::command::{Filter, LayerEffect};

#[derive(Clone, Debug)]
pub(crate) enum LayerFilters {
    One(Filter),
    List(Arc<[Filter]>),
}

impl LayerFilters {
    pub(crate) fn of(effect: &LayerEffect) -> Option<Self> {
        match effect {
            LayerEffect::LayerBlur { sigma } => Some(Self::One(Filter::Blur(*sigma))),
            LayerEffect::Filter(filters) if !filters.is_empty() => {
                Some(Self::List(Arc::clone(filters)))
            }
            _ => None,
        }
    }

    pub(crate) fn steps(&self) -> impl Iterator<Item = &[Filter]> {
        let filters = match self {
            Self::One(filter) => std::slice::from_ref(filter),
            Self::List(filters) => filters,
        };
        filters.chunk_by(|left, right| left.is_color() && right.is_color())
    }
}
