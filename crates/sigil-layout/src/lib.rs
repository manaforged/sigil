//! Taffy-backed layout adapter for `sigil-core`.
//!
//! `sigil-layout` mirrors a `sigil_core::Cx` subtree into a private Taffy
//! tree, computes layout, and writes [`LayoutResult`] extensions back
//! onto the core nodes.

// `criterion` is a dev-dep used only by the `layout_ops` bench. Cargo
// pulls dev-deps into the lib-test target, so keep the dependency
// visible to the unused-crate-dependencies lint.
#[cfg(test)]
use criterion as _;

mod adapter;
pub mod builder_ext;
mod engine;
mod error;
mod intent;
mod measure;
pub mod prelude;
mod result;
pub(crate) mod style_adapter;
mod text_adapter;

pub use builder_ext::LayoutBuilderExt;
pub use engine::LayoutEngine;
pub use error::LayoutError;
pub use intent::LayoutStyle;
pub use measure::TextMeasure;
pub use result::LayoutResult;
pub use text_adapter::TextEngineMeasure;
