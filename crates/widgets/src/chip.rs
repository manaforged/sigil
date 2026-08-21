//! Styled badge / pill widget for log levels, tags, and status indicators.
//!
//! Two log-tailer / kanban friction logs converged on the same shape:
//! a small rounded container holding a short label, tinted by a semantic
//! tone (success / warn / error / info / neutral). This is the smallest
//! widget that closes that gap.
//!
//! ## Theme token mapping (DS1 scale tokens)
//!
//! Each [`Tone`] maps to a Radix-style background/foreground pair on
//! its role scale — step 3 for the background (light-tinted), and the
//! darkest same-hue text step clearing WCAG AA 4.5:1 against it (see
//! [`chip_class`]). This convention keeps status chips legible
//! against any theme surface and makes the success / warn / error chips
//! visually distinct without crossing into the saturated step-9 brand
//! color (reserved for solid buttons).
//!
//! | `Tone`     | Background     | Foreground         |
//! |------------|----------------|--------------------|
//! | `Neutral`  | `bg-gray-3`    | `text-gray-12`     |
//! | `Primary`  | `bg-accent-3`  | `text-accent-12`   |
//! | `Info`     | `bg-info-3`    | `text-info-12`     |
//! | `Success`  | `bg-success-3` | `text-success-12`  |
//! | `Warn`     | `bg-warn-3`    | `text-warn-12`     |
//! | `Error`    | `bg-error-3`   | `text-error-11`    |

use sigil_kernel::TextContent;
use sigil_style::Tone;

mod builder;
pub use builder::ChipBuilder;

/// Soft chip fill + text + border for `tone`.
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

/// Horizontal inner padding in logical pixels.
const CHIP_PADDING_X: f32 = 8.0;
/// Vertical inner padding in logical pixels.
const CHIP_PADDING_Y: f32 = 2.0;

/// Small styled badge / pill — rounded container with a tone-specific
/// background and a short text label.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// chip("ERROR", Tone::Error)
/// chip(status, Tone::Warn).tone_signal(level_signal)
/// chip("Filter", Tone::Info).activate(|| {})
/// ```
///
/// The chip uses `Role::Label` for accessibility — screen readers
/// announce the label text exactly as supplied. `.tone_signal(...)`
/// retints reactively without a remount.
#[must_use]
pub fn chip(label: impl Into<TextContent>, tone: Tone) -> ChipBuilder {
    ChipBuilder::new(label, tone)
}

/// Pill badge — same tone map as [`chip`], fully rounded.
#[must_use]
pub fn badge(label: impl Into<TextContent>, tone: Tone) -> ChipBuilder {
    ChipBuilder::new(label, tone).pill()
}

#[cfg(test)]
#[path = "chip_tests.rs"]
mod tests;
