pub(crate) mod presence;
pub(crate) mod srgb;
pub(crate) mod transform;
use phf as _;
use smallvec as _;
pub use transform::Transform2D;

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
pub mod palette;
pub(crate) mod parser;
pub(crate) mod reactive;
pub(crate) mod resolve;
pub mod scale;
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
pub mod variant;
pub(crate) mod variant_kit;

pub use animator_overlay::{AnimatorOverlay, ColorAtomKind, NumericAtomKind};
pub use atom::{
    AlignKind, BgGradient, ColorAtom, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind,
    GradientAxis, GradientStopKey, JustifyKind, MillisU16, OverflowKind, Px, RenderStyle, Rgba8,
    SPACING_STEP, StyleAtom, TextDecoration, TransitionId,
};
pub use binding::{ReactiveStateSlot, ReactiveStyleAtoms, StyleBinding};
pub(crate) use builder_ext::ClassSource;
pub use builder_ext::NodeStyleExt;
pub(crate) use class_parse::parse_classes;
pub use engine::StyleEngine;
pub use focus_ring::FocusRing;
pub use font_family::FontFamily;
pub use grid::grid;
pub(crate) use grid::{cell_size, swap_cell};
pub use locale::{DateOrder, Direction, Locale};
pub(crate) use merge::merge_classes;
pub(crate) use merge::merge_warning_count_for_component;
pub use merge::{ComponentId, unknown_class_warning_count};
pub use metrics::{
    BORDER_WIDTH, DISABLED_OPACITY, ICON_SIZE, Metrics, OVERLAY_SHADOW, Radius, space,
};
pub use overlay::StyleOverlay;
pub use paint::Paint;
pub(crate) use paint::{PaintTarget, StateHost, StyleSlot};
pub(crate) use parser::parse_class_string;
#[cfg(any(test, fuzzing))]
pub(crate) use parser::parse_stateful_token;
pub use reactive::StyleReactiveEffect;
#[cfg(any(test, feature = "kittest"))]
pub use resolve::unsupported_style_warning_count_for;
pub use resolve::{render_style_for, text_style_for};
pub use size::{ControlSize, LengthPercentage};
pub use state::{StateSlot, StateStyle, StyleState, StyleVariant};
pub use strings::{StringKey, Strings};
pub(crate) use struct_api::Style;
pub(crate) use styled::node::attach_style;
pub use styled::{ColorOrToken, StateStyleBuilder, StateVariants};
pub use text_style::{
    StyledTextEngine, TextFace, TextLineHeight, TextRun, TextRuns, TextStyle, is_typography_atom,
};
pub use theme::{Theme, ThemeBuilder, ThemeValue, TokenLayer, TokenTable, font_family_hint};
pub use token::{ColorKey, Token};
pub use tone::{Tone, ToneContext};
pub use variant_kit::{GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT};

pub const SELECTION_FILL: Rgba8 = Rgba8 {
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
