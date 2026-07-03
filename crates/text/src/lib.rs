//! Owned text engine for Sigil.
//!
//! G1.1 ships the skeleton: skrifa-backed shaper stub, unicode
//! segmentation wrappers, bidi placeholder, and the [`GlyphRun`]
//! output type Slug consumes. The Parley-backed [`engine`],
//! [`paragraph`], and [`cache`] modules stay in place until G1.4
//! rewrites them against the owned shaper (DR-044).
//!
//! See [`docs/decisions/DR-044-g1-owned-text-pipeline.md`].

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use kernel as _;
#[cfg(test)]
use layout as _;
use reactive as _;
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
mod fonts;
mod paragraph;

pub mod bidi;
pub mod glyph;
pub mod segment;

pub(crate) mod shaper;

#[cfg(feature = "shaping-complex")]
pub(crate) mod shaping_complex;

pub use config::TextConfig;
pub use engine::{FontCollection, OwnedTextEngine};
pub use error::TextError;
pub use fonts::{DefaultFonts, set_default_fonts};
pub use paragraph::Paragraph;
pub use shaper::Font;
