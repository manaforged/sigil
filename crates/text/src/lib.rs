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
use sigil_kernel as _;
#[cfg(test)]
use sigil_layout as _;
use sigil_reactive as _;
#[cfg(test)]
use taffy as _;

mod cache;
mod config;
mod engine;
mod error;
mod paragraph;

pub mod bidi;
pub mod glyph;
pub mod segment;

pub(crate) mod shaper;

#[cfg(feature = "shaping-complex")]
pub(crate) mod shaping_complex;

pub use config::TextConfig;
pub use engine::ParleyTextEngine;
pub use error::TextError;
pub use paragraph::Paragraph;
