//! Per-frame paragraph cache.

#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use ahash::{AHashMap, RandomState};
use sigil_style::{TextLineHeight, TextStyle};

use crate::Paragraph;

/// Upper bound on retained paragraphs. Caps memory for the standalone
/// (never-frame-cleared) geometry engine while staying far above the working
/// set of any single frame's text nodes.
const MAX_ENTRIES: usize = 2_048;

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
    style: DiscreteTextStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DiscreteTextStyle {
    font_family_hash: u64,
    font_size_bits: u32,
    font_weight: u16,
    line_height: DiscreteLineHeight,
    letter_spacing_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DiscreteLineHeight {
    FontSizeRelative(u32),
    Absolute(u32),
}

/// One cached paragraph plus the originals required to verify a hit.
///
/// The key alone is a hash; collisions are possible (vanishingly rare
/// for ahash but architecturally unsound to ignore). Each entry retains
/// the original `text` + `style` + `max_width` so `get` can confirm a
/// hit is real and treat a collision as a cache miss.
struct CacheEntry {
    text: String,
    style: TextStyle,
    max_width: Option<f32>,
    paragraph: Paragraph,
}

/// Frame-scoped paragraph cache.
pub(crate) struct ParagraphCache {
    map: AHashMap<CacheKey, CacheEntry>,
}

impl ParagraphCache {
    pub(crate) fn new() -> Self {
        Self {
            map: AHashMap::new(),
        }
    }

    /// Look up a cached paragraph.
    ///
    /// A hit on the hashed key is verified against the original
    /// `(text, style, max_width)` tuple — collisions return `None` so
    /// the caller re-shapes and overwrites the slot.
    pub(crate) fn get(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Option<&Paragraph> {
        let key = Self::key(text, max_width, style);
        let entry = self.map.get(&key)?;
        if entry.text == text && entry.style == *style && widths_match(entry.max_width, max_width) {
            Some(&entry.paragraph)
        } else {
            None
        }
    }

    pub(crate) fn insert(
        &mut self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
        para: Paragraph,
    ) {
        let key = Self::key(text, max_width, style);
        // Bound memory. The per-frame measure engine is cleared every frame, but
        // the standalone geometry engine (used for caret/selection hit-testing)
        // has no frame hook, so without a cap it would retain every distinct
        // string ever hit-tested for the whole session. The cache exists only
        // for short-term reuse, so evicting an arbitrary entry on overflow costs
        // at most a re-shape on the next miss.
        if self.map.len() >= MAX_ENTRIES
            && !self.map.contains_key(&key)
            && let Some(victim) = self.map.keys().next().copied()
        {
            self.map.remove(&victim);
        }
        self.map.insert(
            key,
            CacheEntry {
                text: text.to_owned(),
                style: style.clone(),
                max_width,
                paragraph: para,
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    fn key(text: &str, max_width: Option<f32>, style: &TextStyle) -> CacheKey {
        CacheKey {
            text_hash: hasher().hash_one(text),
            width: DiscreteWidth::from_option(max_width),
            style: DiscreteTextStyle::from(style),
        }
    }
}

/// Two `max_width` values match for cache purposes iff they are both
/// `None` or both `Some(_)` with the same bit pattern. The hash key
/// uses a discretised width so two near-identical floats still collide
/// onto the same slot; verification here uses exact equality so a
/// caller asking for `Some(199.999)` does not silently get the layout
/// computed for `Some(200.0)`.
const fn widths_match(a: Option<f32>, b: Option<f32>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x.to_bits() == y.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

impl From<&TextStyle> for DiscreteTextStyle {
    fn from(style: &TextStyle) -> Self {
        Self {
            font_family_hash: hasher().hash_one(style.font_family.as_str()),
            font_size_bits: style.font_size.to_bits(),
            font_weight: style.font_weight,
            line_height: DiscreteLineHeight::from(style.line_height),
            letter_spacing_bits: style.letter_spacing_em.to_bits(),
        }
    }
}

impl From<TextLineHeight> for DiscreteLineHeight {
    fn from(line_height: TextLineHeight) -> Self {
        match line_height {
            TextLineHeight::FontSizeRelative(value) => Self::FontSizeRelative(value.to_bits()),
            TextLineHeight::Absolute(value) => Self::Absolute(value.to_bits()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "cache_tests.rs"]
mod tests;
