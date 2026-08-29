#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use ahash::{AHashMap, RandomState};
use sigil_style::{TextLineHeight, TextStyle};

use crate::Paragraph;

const MAX_ENTRIES: usize = 2_048;

fn hasher() -> &'static RandomState {
    static HASHER: OnceLock<RandomState> = OnceLock::new();
    HASHER.get_or_init(RandomState::default)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DiscreteWidth(i32);

impl DiscreteWidth {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
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

struct CacheEntry {
    text: String,
    style: TextStyle,
    max_width: Option<f32>,
    paragraph: Paragraph,
}

pub(crate) struct ParagraphCache {
    map: AHashMap<CacheKey, CacheEntry>,
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
#[allow(
    clippy::unwrap_used,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "cache_tests.rs"]
mod tests;
