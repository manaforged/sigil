use phf as _;
use smallvec as _;

#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;

pub mod animator_overlay;
pub mod atom;
pub mod binding;
pub mod builder_ext;
pub mod engine;
pub mod focus_ring;
mod grid;
pub mod merge;
pub mod overlay;
pub mod palette;
pub mod parser;
pub mod reactive;
pub mod resolve;
pub mod scale;
pub mod size;
pub mod state;
pub mod struct_api;
pub mod styled;
pub mod text_style;
pub mod theme;
pub mod tone;
pub mod variant;
pub mod variant_kit;

pub use crate::classes;
pub use animator_overlay::{AnimatorOverlay, ColorAtomKind, NumericAtomKind};
pub use atom::{
    AlignKind, ColorAtom, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, JustifyKind,
    MillisU16, OverflowKind, Px, RenderStyle, Rgba8, SPACING_STEP, StyleAtom, TextDecoration,
    TransitionId,
};
pub use binding::{ReactiveStateSlot, ReactiveStyleAtoms, StyleBinding};
pub use builder_ext::{ClassSource, NodeStyleExt, parse_classes};
pub use engine::StyleEngine;
pub use focus_ring::FocusRing;
pub use grid::grid;
pub use merge::{
    ComponentId, bump_unknown_class_count, merge_classes, merge_warning_count,
    merge_warning_count_for_component, unknown_class_warning_count,
};
pub use overlay::StyleOverlay;
pub use parser::{ParseError, StatefulToken, parse_class_string, parse_stateful_token};
pub use reactive::StyleReactiveEffect;
pub use resolve::{render_style_for, text_style_for, unsupported_style_warning_count_for};
pub use size::{LengthPercentage, Size};
pub use state::{StateSlot, StateStyle, StyleState, StyleVariant};
pub use struct_api::Style;
pub use styled::{
    ColorOrToken, Fill, Flex, Geometry, NodeStyledBuilder, Sizing, Spacing, StateHost,
    StateStyleBuilder, StateVariants, StyleAccess, Styleable, Styled, Transitions, Typography,
    Visual,
};
pub use text_style::{StyledTextEngine, TextLineHeight, TextStyle, is_typography_atom};
pub use theme::{Theme, ThemeBuilder, ThemeValue};
pub use tone::{Tone, ToneContext};
pub use variant_kit::{GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT};

pub mod prelude {
    pub use crate::style::{
        AlignKind, ClassSource, ColorAtom, ColorAtomKind, ColorOrToken, ComponentId, Dimension,
        DisplayKind, FlexDir, FocusRing, JustifyKind, LengthPercentage, MillisU16, NodeStyleExt,
        NodeStyledBuilder, ParseError, Px, ReactiveStyleAtoms, RenderStyle, Rgba8, SPACING_STEP,
        Size, StateSlot, StateStyle, StateStyleBuilder, Style, StyleAccess, StyleAtom,
        StyleBinding, StyleEngine, StyleReactiveEffect, StyleState, StyleVariant, Styleable,
        Styled, StyledTextEngine, TextLineHeight, TextStyle, Theme, ThemeBuilder, ThemeValue,
        TransitionId, classes, merge_classes, merge_warning_count,
        merge_warning_count_for_component, parse_class_string,
    };
}
