//! Owned layout adapter for `sigil-kernel`.
//!
//! `sigil-layout` mirrors a `sigil_kernel::Cx` subtree into an owned
//! layout drive, computes layout, and writes [`LayoutResult`] extensions
//! back onto the core nodes.

#![deny(missing_docs)]
// Dev-deps used only by specific benches/tests. Cargo pulls dev-deps
// into the lib-test target, so keep them visible to the
// unused-crate-dependencies lint.
#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use rayon as _;
#[cfg(test)]
use sigil_reactive as _;
#[cfg(test)]
use taffy as _;

mod adapter;
mod block;
mod block_collapse;
mod block_size;
pub mod builder_ext;
mod cache_key;
mod engine;
mod engine_drive;
mod error;
mod flex;
mod flex_algorithm;
mod flex_lines;
mod forest;
mod intent;
mod measure;
pub mod prelude;
pub mod reactive_position;
mod result;
pub mod style_adapter;
mod style_data;
mod text_adapter;
pub mod text_geometry;
mod text_style;

pub use builder_ext::LayoutBuilderExt;
pub use engine::LayoutEngine;
pub use engine_drive::{LayoutDrive, NodeHandle, ResolvedBox};
pub use error::LayoutError;
pub use intent::LayoutStyle;
pub use measure::TextMeasure;
pub use reactive_position::{LayoutReactiveExt, ReactivePosition, SizeReactive};
pub use result::LayoutResult;
pub use style_data::{
    AlignContent, AlignItems as StyleAlignItems, AlignSelf as StyleAlignSelf, BoxSizing, Dimension,
    Display as StyleDisplay, FlexDirection as StyleFlexDirection, FlexWrap,
    JustifyContent as StyleJustifyContent, LengthPercentage,
    LengthPercentage as StyleLengthPercentage, LengthPercentageAuto,
    LengthPercentageAuto as StyleLengthPercentageAuto, Overflow, Position,
    Position as StylePosition, Rect, Size, StyleData,
};
pub use text_adapter::TextEngineMeasure;
pub use text_geometry::{CaretGeometry, TextGeometry, set_text_geometry, with_text_geometry};
