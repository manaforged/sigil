#![allow(clippy::redundant_pub_crate)]

use std::cell::Cell;
use std::rc::Rc;

use crate::style::TextStyle;
use crate::style::text_identity::{DiscreteTextStyle, hasher};
use ahash::AHashMap;

use crate::text::Paragraph;

const MAX_ENTRIES: usize = 16_384;
const EVICT_DIVISOR: usize = 8;
const MAX_STYLES: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    text_hash: u64,
    width_bits: Option<u32>,
    style: DiscreteTextStyle,
}

struct CacheEntry {
    text: Box<str>,
    style: Rc<TextStyle>,
    paragraph: Paragraph,
    fit: f32,
    used: Cell<u64>,
}

pub(crate) struct ParagraphCache {
    map: AHashMap<CacheKey, CacheEntry>,
    styles: AHashMap<DiscreteTextStyle, Rc<TextStyle>>,
    clock: Cell<u64>,
}

impl ParagraphCache {
    pub(crate) fn new() -> Self {
        Self {
            map: AHashMap::new(),
            styles: AHashMap::new(),
            clock: Cell::new(0),
        }
    }

    pub(crate) fn get(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Option<(&Paragraph, f32)> {
        let entry = self.map.get(&Self::key(text, max_width, style))?;
        if &*entry.text != text || *entry.style != *style {
            return None;
        }
        entry.used.set(self.tick());
        Some((&entry.paragraph, entry.fit))
    }

    pub(crate) fn insert(
        &mut self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
        (para, fit): (Paragraph, f32),
    ) {
        let key = Self::key(text, max_width, style);
        if self.map.len() >= MAX_ENTRIES && !self.map.contains_key(&key) {
            self.evict_least_recent();
        }
        let used = Cell::new(self.tick());
        let style = self.intern(key.style, style);
        self.map.insert(
            key,
            CacheEntry {
                text: text.into(),
                style,
                paragraph: para,
                fit,
                used,
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.styles.clear();
    }

    fn intern(&mut self, identity: DiscreteTextStyle, style: &TextStyle) -> Rc<TextStyle> {
        if let Some(known) = self
            .styles
            .get(&identity)
            .filter(|known| ***known == *style)
        {
            return Rc::clone(known);
        }
        if self.styles.len() >= MAX_STYLES {
            self.styles.clear();
        }
        let interned = Rc::new(style.clone());
        self.styles.insert(identity, Rc::clone(&interned));
        interned
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
