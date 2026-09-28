use std::sync::OnceLock;

use ahash::RandomState;

use crate::style::{TextDecoration, TextLineHeight, TextStyle};

pub(crate) fn hasher() -> &'static RandomState {
    static HASHER: OnceLock<RandomState> = OnceLock::new();
    HASHER.get_or_init(RandomState::default)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DiscreteTextStyle {
    font_family_hash: u64,
    font_size_bits: u32,
    font_weight: u16,
    italic: bool,
    line_height: DiscreteLineHeight,
    letter_spacing_bits: u32,
    tabular_nums: bool,
    decoration: TextDecoration,
}

impl DiscreteTextStyle {
    pub(crate) fn identity(style: &TextStyle) -> u64 {
        hasher().hash_one((Self::from(style), &style.runs))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DiscreteLineHeight {
    FontSizeRelative(u32),
    Absolute(u32),
}

impl From<&TextStyle> for DiscreteTextStyle {
    fn from(style: &TextStyle) -> Self {
        Self {
            font_family_hash: hasher().hash_one(style.font_family.as_str()),
            font_size_bits: style.font_size.to_bits(),
            font_weight: style.font_weight,
            italic: style.italic,
            line_height: DiscreteLineHeight::from(style.line_height),
            letter_spacing_bits: style.letter_spacing_em.to_bits(),
            tabular_nums: style.tabular_nums,
            decoration: style.decoration,
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
