#[cfg(test)]
#[path = "layout/lib_tests.rs"]
mod tests;

#[cfg_attr(
    not(debug_assertions),
    allow(
        dead_code,
        reason = "a release build only reads the audit through inspect or kittest"
    )
)]
pub mod audit;
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
mod orientation;
pub(crate) mod reactive_position;
mod resolve;
mod result;
mod shift;
pub(crate) mod snap;
mod space;
pub(crate) mod style_adapter;
mod style_data;
#[cfg(feature = "scene")]
pub(crate) mod text_adapter;
pub(crate) mod text_geometry;
mod text_style;
mod units;

pub use builder_ext::{column, grid, row};
pub use engine::LayoutEngine;
pub(crate) use engine::{Popup, PopupSide, ViewportOverlay};
#[cfg(feature = "kittest")]
pub use engine_drive::{LayoutDrive, NodeHandle, ResolvedBox};
pub use intent::LayoutStyle;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use measure::FakeMeasure;
pub use measure::TextMeasure;
pub use orientation::Orientation;
pub(crate) use reactive_position::SizeReactive;
pub use reactive_position::{BoundsReactive, ReactivePosition};
pub use result::LayoutResult;
pub use space::Space;
#[cfg(feature = "kittest")]
pub use style_adapter::apply_style_atoms;
pub use style_data::{
    AlignContent, AlignItems, Axes, BoxSizing, Display, Edges, FlexDirection, FlexWrap,
    JustifyContent, Length, Overflow, Position, StyleData, TrackSize,
};
#[cfg(scene_harness)]
pub use text_adapter::TextEngineMeasure;
pub use text_geometry::CaretGeometry;
#[cfg(feature = "kittest")]
pub use text_geometry::text_geometry;
pub(crate) use text_geometry::{TextStyleCapture, with_text_geometry};
#[cfg(feature = "pty")]
pub(crate) use units::mono_cell;
pub(crate) use units::px_sealed::Sealed as IntoPxExt;
pub use units::{IntoPx, cols, em, rows};
