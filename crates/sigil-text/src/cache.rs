//! Per-frame paragraph cache.

#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use ahash::{AHashMap, RandomState};

use crate::Paragraph;

/// Shared deterministic hasher — initialised once, reused everywhere.
fn hasher() -> &'static RandomState {
    static HASHER: OnceLock<RandomState> = OnceLock::new();
    HASHER.get_or_init(RandomState::default)
}

/// Discretised available width for cache keys.
///
/// Rounds to the nearest 0.5 px to avoid float-comparison misses
/// while keeping the cache effective for minor constraint jitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DiscreteWidth(i32);

impl DiscreteWidth {
    #[allow(clippy::cast_possible_truncation)]
    fn from_option(w: Option<f32>) -> Self {
        w.map_or(Self(i32::MAX), |v| Self((v * 2.0) as i32))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    text_hash: u64,
    width: DiscreteWidth,
}

/// Frame-scoped paragraph cache.
pub(crate) struct ParagraphCache {
    map: AHashMap<CacheKey, Paragraph>,
}

impl ParagraphCache {
    pub(crate) fn new() -> Self {
        Self {
            map: AHashMap::new(),
        }
    }

    pub(crate) fn get(&self, text: &str, max_width: Option<f32>) -> Option<&Paragraph> {
        let key = Self::key(text, max_width);
        self.map.get(&key)
    }

    pub(crate) fn insert(&mut self, text: &str, max_width: Option<f32>, para: Paragraph) {
        let key = Self::key(text, max_width);
        self.map.insert(key, para);
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    fn key(text: &str, max_width: Option<f32>) -> CacheKey {
        CacheKey {
            text_hash: hasher().hash_one(text),
            width: DiscreteWidth::from_option(max_width),
        }
    }
}
