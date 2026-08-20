//! Owned text engine for Sigil (DR-044).
//!
//! Shaping runs through `HarfRust` for every script by default
//! (kerning, ligatures, marks); the skrifa-only fallback shaper covers
//! Latin, CJK, and emoji when `HarfRust` returns nothing or the
//! `shaping-complex` feature is off. Around that sit unicode
//! segmentation, bidi resolution, line breaking, font fallback, and a
//! paragraph cache. [`glyph::GlyphRun`] is the output type Slug
//! consumes.
//!
//! See internal decision record DR-044.

#![deny(missing_docs)]
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil_kernel as _;
#[cfg(test)]
use sigil_layout as _;
use sigil_reactive as _;
// `tracing` is only used by the complex-script fallback warning path
// (active when the `shaping-complex` feature is OFF). When ON, the
// warn helper is cfg'd out and tracing would be unused.
#[cfg(feature = "shaping-complex")]
use tracing as _;
// `unicode-script` is consumed inside the `shaper` and (when the
// feature is on) `shaping_complex` modules. The crate-root reference
// here silences `unused_crate_dependencies` for tooling that parses
// the lib.rs prelude before the module bodies.
use unicode_script as _;

mod cache;
mod config;
mod engine;
mod error;
mod font_family;
mod fonts;
mod paragraph;
mod system_fonts;

pub mod bidi;
pub mod color_glyph;
pub mod glyph;
pub mod segment;

pub(crate) mod shaper;

#[cfg(feature = "shaping-complex")]
pub(crate) mod shaping_complex;

pub use config::TextConfig;
pub use engine::{FontCollection, OwnedTextEngine};
pub use error::TextError;
#[doc(hidden)]
pub use fonts::prefer_system_fonts;
pub use fonts::{DefaultFonts, set_default_fonts, use_system_fonts};
pub use paragraph::Paragraph;
pub use shaper::Font;
