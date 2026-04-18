//! Per-frame paragraph cache.

#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use ahash::{AHashMap, RandomState};
use sigil_style::{TextLineHeight, TextStyle};

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
    style: DiscreteTextStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DiscreteTextStyle {
    font_family_hash: u64,
    font_size_bits: u32,
    font_weight: u16,
    line_height: DiscreteLineHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DiscreteLineHeight {
    FontSizeRelative(u32),
    Absolute(u32),
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

    pub(crate) fn get(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Option<&Paragraph> {
        let key = Self::key(text, max_width, style);
        self.map.get(&key)
    }

    pub(crate) fn insert(
        &mut self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
        para: Paragraph,
    ) {
        let key = Self::key(text, max_width, style);
        self.map.insert(key, para);
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

impl From<&TextStyle> for DiscreteTextStyle {
    fn from(style: &TextStyle) -> Self {
        Self {
            font_family_hash: hasher().hash_one(style.font_family.as_str()),
            font_size_bits: style.font_size.to_bits(),
            font_weight: style.font_weight,
            line_height: DiscreteLineHeight::from(style.line_height),
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
