#[cfg(test)]
#[path = "layout/lib_tests.rs"]
mod tests;

kittest_pub! {
    #[cfg_attr(
        not(debug_assertions),
        allow(
            dead_code,
            reason = "a release build only reads the audit through inspect or kittest"
        )
    )]
    #[cfg_attr(
        not(feature = "kittest"),
        allow(unreachable_pub, reason = "public only under kittest")
    )]
    #[doc(hidden)]
    pub mod audit;
}
mod block;
mod block_collapse;
mod block_size;
pub(crate) mod builder_ext;
mod cache_key;
mod engine;
pub(crate) mod engine_drive;
mod flaw;
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
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use engine::LayoutEngine;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use engine::LayoutEngine;
pub(crate) use engine::{Popup, PopupSide, ViewportOverlay};
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use engine_drive::{LayoutDrive, NodeHandle, ResolvedBox};
pub use flaw::Flaw;
pub use intent::LayoutStyle;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use measure::FakeMeasure;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use measure::TextMeasure;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use measure::TextMeasure;
pub use orientation::Orientation;
pub(crate) use reactive_position::SizeReactive;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use reactive_position::{BoundsReactive, ReactivePosition};
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use reactive_position::{BoundsReactive, ReactivePosition};
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use result::LayoutResult;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use result::LayoutResult;
pub use space::Space;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use style_adapter::apply_style_atoms;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use style_data::{AlignContent, Axes, BoxSizing, Edges, FlexDirection, StyleData};
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use style_data::{AlignContent, Axes, BoxSizing, Edges, FlexDirection, StyleData};
pub use style_data::{
    AlignItems, Display, FlexWrap, JustifyContent, Length, Overflow, Position, TrackSize,
};
#[cfg(scene_harness)]
#[doc(hidden)]
pub use text_adapter::TextEngineMeasure;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use text_geometry::CaretGeometry;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use text_geometry::CaretGeometry;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use text_geometry::text_geometry;
pub(crate) use text_geometry::{TextStyleCapture, with_text_geometry};
#[cfg(feature = "pty")]
pub(crate) use units::mono_cell;
pub use units::{cols, em, rows};
pub(crate) use units::{positive_px, px_or_zero};
