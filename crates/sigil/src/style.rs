pub(crate) mod presence;
pub(crate) mod projection;
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
pub(crate) mod diagnostic;
pub(crate) mod engine;
pub(crate) mod focus_ring;
pub(crate) mod font_family;
mod grid;
pub(crate) mod image_paint;
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
mod text_scale;
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
pub(crate) use animator_overlay::NumericAtomKind;
pub(crate) use animator_overlay::{ColorAtomKind, Family};
pub(crate) use atom::Px;
pub(crate) use atom::SPACING_STEP;
pub use atom::TextDecoration;
pub use atom::{
    Background, ClipShape, Color, ColorAtom, Conic, Elevation, Gradient, GradientAxis, MaskImage,
    MaskSize, Radial, RadialExtent,
};
kittest_pub! {
    #[doc(hidden)]
    pub use atom::{
        Backface, BgGradient, BorderStyle, FilterAtom, GradientShape, GradientStopKey, MaskAtom,
        MillisU16, PaintAtom, PaintStyle, RenderStyle, SpaceAtom, SpaceStyle, StopSlot, StyleAtom,
        TransitionId,
    };
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
pub use focus_ring::FocusRing;
pub use font_family::FontFamily;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use grid::cell;
pub(crate) use grid::{cell_size, swap_cell};
pub use locale::Direction;
pub use locale::{DateOrder, Locale};
pub(crate) use merge::merge_classes;
#[cfg(test)]
pub(crate) use merge::merge_warning_count_for_component;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use merge::unknown_class_warning_count;
pub use metrics::{BORDER_WIDTH, Layer, Metrics, Radius, space};
pub(crate) use metrics::{DIMMED, DISABLED_OPACITY, ICON_SIZE};
pub(crate) use overlay::StyleOverlay;
pub use paint::Paint;
pub(crate) use paint::{PaintTarget, StyleSlot};
pub(crate) use parser::parse_class_string;
#[cfg(any(test, fuzzing))]
pub(crate) use parser::parse_stateful_token;
pub(crate) use reactive::StyleReactiveEffect;
pub use text_scale::TextRole;
pub use text_scale::text_scale;
kittest_pub! {
    #[doc(hidden)]
    pub use resolve::render_style_for;
}
pub(crate) use resolve::text_style_for;
pub use scale::{ColorScale, ScaleRole};
pub use size::ControlSize;
pub(crate) use state::StyleVariant;
kittest_pub! {
    #[doc(hidden)]
    pub use state::{StateSlot, StateStyle, StyleState};
}
pub use strings::{StringKey, Strings};
pub(crate) use struct_api::Style;
pub use styled::StateStyleBuilder;
pub use styled::States;
pub(crate) use styled::node::attach_style;
pub use text_style::TextStyle;
pub(crate) use text_style::is_typography_atom;
pub use text_style::{TextFace, TextLineHeight, TextRun, TextRuns};
pub use theme::{Theme, ThemeValue};
pub use token::Token;
pub use tone::Tone;
kittest_pub! {
    #[doc(hidden)]
    pub use tone::ToneContext;
}
pub(crate) use variant_kit::{GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT};

/// Gives the fill color for selected text.
pub(crate) const SELECTION_FILL: Color = Color {
    r: 56,
    g: 118,
    b: 224,
    a: 130,
};

kittest_pub! {
    #[doc(hidden)]
    pub use crate::hal::geometry::Rgba;
}

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
