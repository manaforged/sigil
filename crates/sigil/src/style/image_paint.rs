use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, PoisonError};

use crate::kernel::ImageSource;

const MAX_IMAGES: usize = 512;

#[derive(Default)]
struct Registry {
    images: HashMap<u64, ImageSource>,
    order: VecDeque<u64>,
}

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

pub(crate) fn register(source: ImageSource) -> u64 {
    let id = source.id();
    let mut guard = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    let registry = guard.get_or_insert_with(Registry::default);
    if registry.images.insert(id, source).is_none() {
        registry.order.push_back(id);
    }
    while registry.order.len() > MAX_IMAGES {
        if let Some(oldest) = registry.order.pop_front() {
            registry.images.remove(&oldest);
        }
    }
    id
}

pub(crate) fn lookup(id: u64) -> Option<ImageSource> {
    let guard = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    guard.as_ref()?.images.get(&id).cloned()
}
