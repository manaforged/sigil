#![allow(clippy::redundant_pub_crate)]

use std::cell::Cell;
use std::rc::Rc;

use crate::style::TextStyle;
use crate::style::text_identity::{DiscreteTextStyle, hasher};
use ahash::AHashMap;
use smallvec::SmallVec;

use crate::cache::Bounded;

use crate::text::Paragraph;

const MAX_ENTRIES: usize = 16_384;
const EVICT_DIVISOR: usize = 8;
const MAX_STYLES: usize = 1_024;
const MAX_WIDTHS_PER_TEXT: usize = 8;
pub(crate) const WRAP_SLACK: f32 = 1e-3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct WrapBounds {
    pub(crate) min_nowrap: f32,
    pub(crate) min_wrap: f32,
}

impl Default for WrapBounds {
    fn default() -> Self {
        Self::OPEN
    }
}

impl WrapBounds {
    pub(crate) const OPEN: Self = Self {
        min_nowrap: 0.0,
        min_wrap: f32::INFINITY,
    };

    pub(crate) fn contains(self, limit: f32) -> bool {
        self.min_nowrap - limit <= WRAP_SLACK
            && (!self.min_wrap.is_finite() || self.min_wrap - limit > WRAP_SLACK)
    }
}

pub(crate) struct CachedParagraph {
    pub(crate) paragraph: Paragraph,
    pub(crate) bounds: WrapBounds,
}

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
    bounds: WrapBounds,
    used: Cell<u64>,
}

type WidthBuckets = AHashMap<(u64, DiscreteTextStyle), SmallVec<[Option<u32>; 4]>>;

pub(crate) struct ParagraphCache {
    map: AHashMap<CacheKey, CacheEntry>,
    by_text: WidthBuckets,
    styles: Bounded<DiscreteTextStyle, Rc<TextStyle>, MAX_STYLES>,
    clock: Cell<u64>,
}

impl ParagraphCache {
    pub(crate) fn new() -> Self {
        Self {
            map: AHashMap::new(),
            by_text: AHashMap::new(),
            styles: Bounded::default(),
            clock: Cell::new(0),
        }
    }

    pub(crate) fn lookup(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Option<&Paragraph> {
        let key = Self::key(text, max_width, style);
        if let Some(entry) = self.map.get(&key)
            && &*entry.text == text
            && *entry.style == *style
        {
            entry.used.set(self.tick());
            return Some(&entry.paragraph);
        }
        let limit = max_width?;
        let widths = self.by_text.get(&(key.text_hash, key.style))?;
        for &width_bits in widths {
            if width_bits == key.width_bits {
                continue;
            }
            let candidate = CacheKey { width_bits, ..key };
            let Some(entry) = self.map.get(&candidate) else {
                continue;
            };
            if &*entry.text == text && *entry.style == *style && entry.bounds.contains(limit) {
                entry.used.set(self.tick());
                return Some(&entry.paragraph);
            }
        }
        None
    }

    pub(crate) fn insert(
        &mut self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
        value: CachedParagraph,
    ) {
        let key = Self::key(text, max_width, style);
        if self.map.len() >= MAX_ENTRIES && !self.map.contains_key(&key) {
            self.evict_least_recent();
        }
        let used = Cell::new(self.tick());
        let style = self.intern(key.style, style);
        self.record_width(key);
        self.map.insert(
            key,
            CacheEntry {
                text: text.into(),
                style,
                paragraph: value.paragraph,
                bounds: value.bounds,
                used,
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.by_text.clear();
        self.styles.clear();
    }

    fn record_width(&mut self, key: CacheKey) {
        let widths = self.by_text.entry((key.text_hash, key.style)).or_default();
        if widths.contains(&key.width_bits) {
            return;
        }
        if widths.len() >= MAX_WIDTHS_PER_TEXT {
            widths.remove(0);
        }
        widths.push(key.width_bits);
    }

    fn intern(&mut self, identity: DiscreteTextStyle, style: &TextStyle) -> Rc<TextStyle> {
        if let Some(known) = self
            .styles
            .get(&identity)
            .filter(|known| ***known == *style)
        {
            return Rc::clone(known);
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
        let map = &self.map;
        self.by_text.retain(|&(text_hash, style), widths| {
            widths.retain(|width_bits| {
                map.contains_key(&CacheKey {
                    text_hash,
                    width_bits: *width_bits,
                    style,
                })
            });
            !widths.is_empty()
        });
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
