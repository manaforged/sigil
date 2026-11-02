pub(crate) mod presence;
pub(crate) mod srgb;
pub(crate) mod transform;
use phf as _;
use smallvec as _;
kittest_pub! {
    #[doc(hidden)]
    pub use transform::Transform2D;
}

#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;

pub(crate) mod animator_overlay;
pub(crate) mod atom;
pub(crate) mod binding;
pub(crate) mod builder_ext;
pub(crate) mod class_parse;
pub(crate) mod engine;
pub(crate) mod focus_ring;
pub(crate) mod font_family;
mod grid;
mod install;
pub(crate) mod intern;
pub(crate) mod locale;
pub(crate) mod merge;
pub(crate) mod metrics;
pub(crate) mod overlay;
mod paint;
pub(crate) mod palette;
pub(crate) mod parser;
pub(crate) mod reactive;
pub(crate) mod resolve;
kittest_pub! {
    /// The color scales of the default themes.
    #[doc(hidden)]
    pub mod scale;
}
pub(crate) mod size;
pub(crate) mod state;
pub(crate) mod strings;
pub(crate) mod struct_api;
pub(crate) mod styled;
pub(crate) mod text_identity;
pub(crate) mod text_style;
pub(crate) mod theme;
pub(crate) mod token;
pub(crate) mod tone;
pub(crate) mod variant;
pub(crate) mod variant_kit;

pub(crate) use animator_overlay::AnimatorOverlay;
pub(crate) use animator_overlay::{ColorAtomKind, NumericAtomKind};
pub(crate) use atom::Px;
pub(crate) use atom::SPACING_STEP;
pub(crate) use atom::TextDecoration;
pub use atom::{ColorAtom, Elevation, Rgba8};
kittest_pub! {
    #[doc(hidden)]
    pub use atom::{BgGradient, GradientAxis, GradientStopKey, MillisU16, RenderStyle, StyleAtom, TransitionId};
}
pub(crate) use binding::ReactiveStateSlot;
#[cfg(not(feature = "kittest"))]
pub(crate) use binding::{ReactiveStyleAtoms, StyleBinding};
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use binding::{ReactiveStyleAtoms, StyleBinding};
pub(crate) use builder_ext::ClassSource;
pub(crate) use class_parse::parse_classes;
kittest_pub! {
    #[doc(hidden)]
    pub use engine::StyleEngine;
}
kittest_pub! {
    #[doc(hidden)]
    pub use focus_ring::FocusRing;
}
pub use font_family::FontFamily;
pub use grid::grid;
pub(crate) use grid::{cell_size, swap_cell};
pub use locale::Direction;
kittest_pub! {
    #[doc(hidden)]
    pub use locale::{DateOrder, Locale};
}
pub(crate) use merge::merge_classes;
#[cfg(test)]
pub(crate) use merge::merge_warning_count_for_component;
#[cfg(any(test, feature = "kittest"))]
#[doc(hidden)]
pub use merge::unknown_class_warning_count;
pub use metrics::{BORDER_WIDTH, Metrics, Radius, space};
pub(crate) use metrics::{DISABLED_OPACITY, ICON_SIZE, OVERLAY_SHADOW};
pub(crate) use overlay::StyleOverlay;
pub use paint::Paint;
pub(crate) use paint::{PaintTarget, StyleSlot};
pub(crate) use parser::parse_class_string;
#[cfg(any(test, fuzzing))]
pub(crate) use parser::parse_stateful_token;
pub(crate) use reactive::StyleReactiveEffect;
#[cfg(any(test, feature = "kittest"))]
pub use resolve::unsupported_style_warning_count_for;
kittest_pub! {
    #[doc(hidden)]
    pub use resolve::{render_style_for, text_style_for};
}
pub use size::ControlSize;
pub(crate) use state::StyleVariant;
kittest_pub! {
    #[doc(hidden)]
    pub use state::{StateSlot, StateStyle, StyleState};
}
pub use strings::StringKey;
kittest_pub! {
    #[doc(hidden)]
    pub use strings::Strings;
}
pub(crate) use struct_api::Style;
pub use styled::StateVariants;
pub(crate) use styled::node::attach_style;
kittest_pub! {
    #[doc(hidden)]
    pub use styled::StateStyleBuilder;
}
#[cfg(all(feature = "scene", not(feature = "kittest")))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use text_style::StyledTextEngine;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use text_style::StyledTextEngine;
pub use text_style::TextStyle;
pub(crate) use text_style::is_typography_atom;
pub(crate) use text_style::{TextFace, TextRuns};
kittest_pub! {
    #[doc(hidden)]
    pub use text_style::{TextLineHeight, TextRun};
}
pub use theme::{Theme, ThemeValue};
pub use token::Token;
pub use tone::Tone;
kittest_pub! {
    #[doc(hidden)]
    pub use tone::ToneContext;
}
pub(crate) use variant_kit::{GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT};

/// Gives the fill color for selected text.
pub(crate) const SELECTION_FILL: Rgba8 = Rgba8 {
    r: 56,
    g: 118,
    b: 224,
    a: 130,
};

pub use crate::hal::geometry::Color;

#[cfg(test)]
#[path = "style/resolution_tests.rs"]
mod resolution_tests;

#[cfg(test)]
#[path = "style/grammar_tests.rs"]
mod grammar_tests;

#[cfg(test)]
#[path = "style/merge_tests.rs"]
mod merge_tests;

#[cfg(test)]
#[path = "style/reactive_tests.rs"]
mod reactive_tests;
