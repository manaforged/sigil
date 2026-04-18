//! Taffy-backed layout adapter for `sigil-core`.
//!
//! `sigil-layout` mirrors a `sigil_core::Cx` subtree into a private Taffy
//! tree, computes layout, and writes [`LayoutResult`] extensions back
//! onto the core nodes.

// Dev-deps used only by specific benches/tests. Cargo pulls dev-deps
// into the lib-test target, so keep them visible to the
// unused-crate-dependencies lint.
#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil_reactive as _;

mod adapter;
pub mod builder_ext;
mod cache_key;
mod engine;
mod error;
mod intent;
mod measure;
pub mod prelude;
mod result;
pub(crate) mod style_adapter;
mod text_adapter;
mod text_style;

pub use builder_ext::LayoutBuilderExt;
pub use engine::LayoutEngine;
pub use error::LayoutError;
pub use intent::LayoutStyle;
pub use measure::TextMeasure;
pub use result::LayoutResult;
pub use text_adapter::TextEngineMeasure;
