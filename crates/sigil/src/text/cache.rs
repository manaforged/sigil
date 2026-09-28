#![allow(clippy::redundant_pub_crate)]

use std::cell::Cell;

use crate::style::TextStyle;
use crate::style::text_identity::{DiscreteTextStyle, hasher};
use ahash::AHashMap;

use crate::text::Paragraph;

const MAX_ENTRIES: usize = 16_384;
const EVICT_DIVISOR: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    text_hash: u64,
    width_bits: Option<u32>,
    style: DiscreteTextStyle,
}

struct CacheEntry {
    text: String,
    style: TextStyle,
    paragraph: Paragraph,
    used: Cell<u64>,
}

pub(crate) struct ParagraphCache {
    map: AHashMap<CacheKey, CacheEntry>,
    clock: Cell<u64>,
}

impl ParagraphCache {
    pub(crate) fn new() -> Self {
        Self {
            map: AHashMap::new(),
            clock: Cell::new(0),
        }
    }

    pub(crate) fn get(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Option<&Paragraph> {
        let entry = self.map.get(&Self::key(text, max_width, style))?;
        if entry.text != text || entry.style != *style {
            return None;
        }
        entry.used.set(self.tick());
        Some(&entry.paragraph)
    }

    pub(crate) fn insert(
        &mut self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
        para: Paragraph,
    ) {
        let key = Self::key(text, max_width, style);
        if self.map.len() >= MAX_ENTRIES && !self.map.contains_key(&key) {
            self.evict_least_recent();
        }
        let used = Cell::new(self.tick());
        self.map.insert(
            key,
            CacheEntry {
                text: text.to_owned(),
                style: style.clone(),
                paragraph: para,
                used,
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    fn tick(&self) -> u64 {
        let now = self.clock.get().wrapping_add(1);
        self.clock.set(now);
        now
    }

    fn evict_least_recent(&mut self) {
        let mut ages: Vec<u64> = self.map.values().map(|entry| entry.used.get()).collect();
        let batch = (ages.len() / EVICT_DIVISOR).max(1);
        let (_, cutoff, _) = ages.select_nth_unstable(batch - 1);
        let cutoff = *cutoff;
        self.map.retain(|_, entry| entry.used.get() > cutoff);
    }

    fn key(text: &str, max_width: Option<f32>, style: &TextStyle) -> CacheKey {
        CacheKey {
            text_hash: hasher().hash_one(text),
            width_bits: max_width.map(f32::to_bits),
            style: DiscreteTextStyle::from(style),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "cache_tests.rs"]
mod tests;
