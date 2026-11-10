use std::collections::HashSet;
use std::hash::Hash;
use std::sync::{Mutex, OnceLock, PoisonError};

use crate::style::{GradientStopKey, StyleAtom};

type InternCache<T> = OnceLock<Mutex<HashSet<&'static [T]>>>;

pub(crate) trait Interned: Hash + Eq + Send + Sync + 'static + Sized {
    fn cache() -> &'static InternCache<Self>;
}

impl Interned for StyleAtom {
    fn cache() -> &'static InternCache<Self> {
        static CACHE: InternCache<StyleAtom> = OnceLock::new();
        &CACHE
    }
}

impl Interned for GradientStopKey {
    fn cache() -> &'static InternCache<Self> {
        static CACHE: InternCache<GradientStopKey> = OnceLock::new();
        &CACHE
    }
}

pub(crate) fn intern<T: Interned>(items: impl Into<Box<[T]>>) -> &'static [T] {
    let items: Box<[T]> = items.into();
    let mut cache = T::cache()
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(interned) = cache.get(&*items).copied() {
        return interned;
    }
    let interned: &'static [T] = Box::leak(items);
    cache.insert(interned);
    interned
}
