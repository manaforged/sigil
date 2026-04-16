//! Parley-backed text engine for Sigil.
//!
//! This crate wraps [Parley] behind the [`sigil_hal::TextEngine`] trait.
//! The layout-specific [`TextMeasure`](sigil_layout::TextMeasure) adapter
//! lives in `sigil-layout` as [`TextEngineMeasure`](sigil_layout::TextEngineMeasure).
//!
//! [Parley]: https://github.com/linebender/parley

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil_core as _;
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

pub use config::TextConfig;
pub use engine::ParleyTextEngine;
pub use error::TextError;
pub use paragraph::Paragraph;
