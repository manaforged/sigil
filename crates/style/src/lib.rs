//! Utility-class, theme, and reactive style layer for Sigil.

// Step 1 wires the full dependency surface before the phase fills the
// modules. Keep those crates visible to `unused_crate_dependencies`.
use phf as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use smallvec as _;

#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;

pub mod atom;
pub mod binding;
pub mod builder_ext;
pub mod engine;
pub mod focus_ring;
pub mod merge;
pub mod palette;
pub mod parser;
pub mod reactive;
pub mod state;
pub mod struct_api;
pub mod styled;
pub mod text_style;
pub mod theme;
pub mod variant;

pub use atom::{
    AlignKind, ColorAtom, Dimension, DisplayKind, FlexDir, JustifyKind, MillisU16, OverflowKind,
    Px, RenderStyle, Rgba8, SPACING_STEP, StyleAtom, TransitionId,
};
pub use binding::{ReactiveStyleAtoms, StyleBinding};
pub use builder_ext::{ClassSource, NodeStyleExt};
pub use engine::StyleEngine;
pub use focus_ring::FocusRing;
pub use merge::{
    ComponentId, merge_classes, merge_warning_count, merge_warning_count_for_component,
};
pub use parser::{ParseError, parse_class_string};
pub use reactive::StyleReactiveEffect;
pub use sigil_style_macros::classes;
pub use state::{StateSlot, StateStyle, StyleState, StyleVariant};
pub use struct_api::Style;
pub use styled::{
    ColorOrToken, Fill, Flex, Geometry, NodeStyledBuilder, Sizing, Spacing, StateHost,
    StateStyleBuilder, StateVariants, StyleAccess, Styleable, Styled, Transitions, Typography,
    Visual,
};
pub use text_style::{StyledTextEngine, TextLineHeight, TextStyle};
pub use theme::{Theme, ThemeBuilder, ThemeValue};

/// Common imports for applications using Sigil styling.
pub mod prelude {
    pub use crate::{
        AlignKind, ClassSource, ColorAtom, ColorOrToken, ComponentId, Dimension, DisplayKind,
        FlexDir, FocusRing, JustifyKind, MillisU16, NodeStyleExt, NodeStyledBuilder, ParseError,
        Px, ReactiveStyleAtoms, RenderStyle, Rgba8, SPACING_STEP, StateSlot, StateStyle,
        StateStyleBuilder, Style, StyleAccess, StyleAtom, StyleBinding, StyleEngine,
        StyleReactiveEffect, StyleState, StyleVariant, Styleable, Styled, StyledTextEngine,
        TextLineHeight, TextStyle, Theme, ThemeBuilder, ThemeValue, TransitionId, classes,
        merge_classes, merge_warning_count, merge_warning_count_for_component, parse_class_string,
    };
}
