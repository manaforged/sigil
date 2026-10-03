use crate::kernel::TextContent;
use crate::style::{BORDER_WIDTH, StyleAtom, Token, Tone};

mod builder;
pub use builder::ChipBuilder;

static NEUTRAL: &[StyleAtom] = &[
    Token::Gray3.bg(),
    Token::Gray12.text(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Gray6.border(),
];
static PRIMARY: &[StyleAtom] = &[
    Token::Accent3.bg(),
    Token::Accent12.text(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Accent6.border(),
];
static INFO: &[StyleAtom] = &[
    Token::Info3.bg(),
    Token::Info12.text(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Info6.border(),
];
static SUCCESS: &[StyleAtom] = &[
    Token::Success3.bg(),
    Token::Success12.text(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Success6.border(),
];
static WARN: &[StyleAtom] = &[
    Token::Warn3.bg(),
    Token::Warn12.text(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Warn6.border(),
];
static ERROR: &[StyleAtom] = &[
    Token::Error3.bg(),
    Token::Error11.text(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Error6.border(),
];

pub(crate) const fn chip_class(tone: Tone) -> &'static [StyleAtom] {
    match tone {
        Tone::Neutral => NEUTRAL,
        Tone::Primary => PRIMARY,
        Tone::Info => INFO,
        Tone::Success => SUCCESS,
        Tone::Warn => WARN,
        Tone::Error => ERROR,
    }
}

#[must_use]
pub fn chip(label: impl Into<TextContent>, tone: Tone) -> ChipBuilder {
    ChipBuilder::new(label, tone)
}

#[cfg(test)]
#[path = "chip_tests.rs"]
mod tests;
