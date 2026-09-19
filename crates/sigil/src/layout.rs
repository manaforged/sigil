#[cfg(test)]
#[path = "layout/lib_tests.rs"]
mod tests;

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
mod flex_pack;
mod forest;
mod grid;
mod intent;
mod measure;
pub mod prelude;
pub mod reactive_position;
mod resolve;
mod result;
pub mod style_adapter;
mod style_data;
mod text_adapter;
pub mod text_geometry;
mod text_style;

pub use builder_ext::LayoutBuilderExt;
pub use engine::LayoutEngine;
pub(crate) use engine::ViewportOverlay;
pub use engine_drive::{LayoutDrive, NodeHandle, ResolvedBox};
pub use error::LayoutError;
pub use intent::LayoutStyle;
pub use measure::TextMeasure;
pub use reactive_position::{BoundsReactive, LayoutReactiveExt, ReactivePosition, SizeReactive};
pub use result::LayoutResult;
pub use style_data::{
    AlignContent, AlignItems as StyleAlignItems, AlignSelf as StyleAlignSelf, BoxSizing, Dimension,
    Display as StyleDisplay, FlexDirection as StyleFlexDirection, FlexWrap,
    JustifyContent as StyleJustifyContent, LengthPercentage,
    LengthPercentage as StyleLengthPercentage, LengthPercentageAuto,
    LengthPercentageAuto as StyleLengthPercentageAuto, Overflow, Position,
    Position as StylePosition, Rect, Size, StyleData, TrackSize,
};
pub use text_adapter::TextEngineMeasure;
pub use text_geometry::{
    CaretGeometry, TextGeometry, TextStyleCapture, set_text_geometry, with_text_geometry,
};
