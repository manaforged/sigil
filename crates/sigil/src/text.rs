#[cfg(test)]
#[path = "text/lib_tests.rs"]
mod tests;
#[cfg(feature = "shaping-complex")]
use tracing as _;
use unicode_script as _;

mod cache;
mod config;
mod engine;
mod error;
mod font_family;
mod fonts;
mod numeric_run;
mod paragraph;
mod script_fonts;
mod system_fonts;

pub use crate::bidi;
pub(crate) mod color_glyph;
mod colr_paint;
mod colr_radial;
pub(crate) mod glyph;

pub(crate) mod shaper;

#[cfg(feature = "shaping-complex")]
pub(crate) mod shaping_complex;

pub use config::TextConfig;
#[cfg(any(test, feature = "kittest"))]
pub use engine::FontCollection;
pub use engine::OwnedTextEngine;
pub use error::TextError;
#[cfg(feature = "desktop")]
pub(crate) use fonts::on_fonts_changed;
pub use fonts::prefer_system_fonts;
pub use fonts::{DefaultFonts, set_default_fonts, use_system_fonts};
pub(crate) use paragraph::FaceSpec;
pub use paragraph::Paragraph;
pub use shaper::Font;

pub use crate::hal::{ParagraphMetrics, TextEngine};
