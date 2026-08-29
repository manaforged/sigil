#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
use sigil_reactive as _;
#[cfg(feature = "shaping-complex")]
use tracing as _;
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
pub use fonts::{DefaultFonts, SansWeight, set_default_fonts, use_system_fonts};
pub use paragraph::Paragraph;
pub use shaper::Font;
