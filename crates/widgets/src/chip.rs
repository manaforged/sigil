use sigil_kernel::TextContent;
use sigil_style::Tone;

mod builder;
pub use builder::ChipBuilder;

pub(crate) const fn chip_class(tone: Tone) -> &'static str {
    match tone {
        Tone::Neutral => "bg-gray-3 text-gray-12 border border-gray-6",
        Tone::Primary => "bg-accent-3 text-accent-12 border border-accent-6",
        Tone::Info => "bg-info-3 text-info-12 border border-info-6",
        Tone::Success => "bg-success-3 text-success-12 border border-success-6",
        Tone::Warn => "bg-warn-3 text-warn-12 border border-warn-6",
        Tone::Error => "bg-error-3 text-error-11 border border-error-6",
    }
}

const CHIP_PADDING_X: f32 = 8.0;
const CHIP_PADDING_Y: f32 = 2.0;

#[must_use]
pub fn chip(label: impl Into<TextContent>, tone: Tone) -> ChipBuilder {
    ChipBuilder::new(label, tone)
}

#[must_use]
pub fn badge(label: impl Into<TextContent>, tone: Tone) -> ChipBuilder {
    ChipBuilder::new(label, tone).pill()
}

#[cfg(test)]
#[path = "chip_tests.rs"]
mod tests;
