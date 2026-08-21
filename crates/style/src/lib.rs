//! Utility-class, theme, and reactive style layer for Sigil.
//!
//! # Resolution precedence (DR-056 D4)
//!
//! Sigil has no CSS specificity. A node's resolved style is built by
//! deterministic layering — later layers win, and within a layer
//! conflicts resolve last-wins per slot (shorthands like `p-4` expand
//! to per-side slots first; see [`merge`]):
//!
//! 1. **Theme tokens** — the ambient [`Theme`] (subtree-scoped via the
//!    node-extension mechanism; nearest ancestor wins).
//! 2. **Variant base** — a widget's [`variant_kit`] atoms (e.g.
//!    `SOLID_VARIANT` base).
//! 3. **Variant state slot** — the variant's atoms for the active
//!    [`StateSlot`] (disabled > pressed > hovered > focused > rest).
//! 4. **Static classes** — `classes!` / `.class("…")` literals.
//! 5. **Dynamic classes** — runtime-built atom lists.
//! 6. **Reactive classes** — `style_reactive` / `class_when` signal
//!    output.
//! 7. **State overlay** — fluent `.hover(…)` / `.focus(…)` /
//!    `.pressed(…)` / `.disabled(…)` builders; interaction-local, never
//!    transmitted to descendants.
//!
//! # Inheritance (DR-056 D1)
//!
//! Matching the CSS inherited-property set: **text color** and
//! **typography** (font family, size, weight, line height) cascade from
//! ancestors to descendant text nodes, own-atom-wins. Typography
//! resolution is shared between layout measurement and renderer paint
//! ([`TextStyle::layered_over_binding`]), so text always paints at the
//! size it measured. Everything else (box model, flex, radius, opacity,
//! overflow, transitions) is non-inherited, also matching CSS.

#![deny(missing_docs)]
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
#[cfg(test)]
use sigil as _;

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

pub use animator_overlay::{AnimatorOverlay, ColorAtomKind, NumericAtomKind};
pub use atom::{
    AlignKind, ColorAtom, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, JustifyKind,
    MillisU16, OverflowKind, Px, RenderStyle, Rgba8, SPACING_STEP, StyleAtom, TextDecoration,
    TransitionId,
};
pub use binding::{ReactiveStateSlot, ReactiveStyleAtoms, StyleBinding};
pub use builder_ext::{ClassSource, NodeStyleExt};
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
pub use sigil_style_macros::classes;
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
pub use variant_kit::{SOFT_VARIANT, SOLID_VARIANT};

/// Common imports for applications using Sigil styling.
pub mod prelude {
    pub use crate::{
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
