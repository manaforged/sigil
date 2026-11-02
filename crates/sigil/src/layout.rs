#[cfg(test)]
#[path = "layout/lib_tests.rs"]
mod tests;

#[cfg(any(
    test,
    feature = "kittest",
    all(feature = "inspect", any(feature = "desktop", feature = "terminal"))
))]
pub mod align;
mod block;
mod block_collapse;
mod block_size;
pub(crate) mod builder_ext;
mod cache_key;
mod engine;
pub(crate) mod engine_drive;
mod flex;
mod flex_algorithm;
mod flex_lines;
mod flex_pack;
mod forest;
mod grid;
mod grid_tracks;
mod intent;
mod measure;
pub(crate) mod reactive_position;
mod resolve;
mod result;
pub(crate) mod snap;
pub(crate) mod style_adapter;
mod style_data;
#[cfg(feature = "scene")]
pub(crate) mod text_adapter;
pub(crate) mod text_geometry;
mod text_style;
mod units;

pub use builder_ext::{column, row};
pub use engine::LayoutEngine;
pub(crate) use engine::{Popup, PopupSide, ViewportOverlay};
#[cfg(any(test, feature = "kittest"))]
pub use engine_drive::{LayoutDrive, NodeHandle, ResolvedBox};
pub use intent::LayoutStyle;
#[cfg(any(test, feature = "kittest"))]
#[doc(hidden)]
pub use measure::FakeMeasure;
pub use measure::TextMeasure;
pub(crate) use reactive_position::SizeReactive;
pub use reactive_position::{BoundsReactive, ReactivePosition};
pub use result::LayoutResult;
#[cfg(any(test, feature = "kittest"))]
pub use style_adapter::apply_style_atoms;
pub use style_data::{
    AlignContent, AlignItems, Axes, BoxSizing, Dimension, Display, Edges, FlexDirection, FlexWrap,
    JustifyContent, Overflow, Position, StyleData, TrackSize,
};
#[cfg(all(feature = "scene", any(test, feature = "kittest")))]
pub use text_adapter::TextEngineMeasure;
pub use text_geometry::CaretGeometry;
#[cfg(any(test, feature = "kittest"))]
pub use text_geometry::set_text_geometry;
pub(crate) use text_geometry::{TextStyleCapture, with_text_geometry};
pub use units::{IntoPx, cols, em, rows};
