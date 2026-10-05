use crate::kernel::TextContent;
use smallvec::smallvec;

use crate::style::class_parse::intern_atoms;
use crate::style::{BORDER_WIDTH, StyleAtom, Tone};

mod builder;
pub use builder::ChipBuilder;

pub(crate) fn chip_class(tone: Tone) -> &'static [StyleAtom] {
    intern_atoms(smallvec![
        tone.token(3).bg(),
        tone.token(12).text(),
        StyleAtom::BorderWidth(BORDER_WIDTH),
        tone.token(6).border(),
    ])
}

#[must_use]
pub fn chip(label: impl Into<TextContent>, tone: Tone) -> ChipBuilder {
    ChipBuilder::new(label, tone)
}

#[cfg(test)]
#[path = "chip_tests.rs"]
mod tests;
