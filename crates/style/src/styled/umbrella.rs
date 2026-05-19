//! The `Styled` umbrella — a single trait alias-style super-trait that
//! collects every per-category fluent surface (`Spacing`, `Sizing`,
//! `Flex`, `Fill`, `Typography`, `Geometry`, `Visual`, `Transitions`,
//! `StateVariants`) into one bound for downstream "anything styleable"
//! call sites.
//!
//! Implementors only ever write `impl StyleAccess for X` — the
//! per-category traits and this umbrella both fall out via blanket impls.
//! See DR-049 for the locked vocabulary.

use super::{
    Fill, Flex, Geometry, Sizing, Spacing, StateVariants, StyleAccess, Transitions, Typography,
    Visual,
};

/// Unified fluent styling surface — the umbrella over every per-category
/// trait. See DR-049 for the locked vocabulary + semantics.
///
/// `Styled` is the bound to use when a function wants "anything that
/// implements the full fluent surface." It is satisfied automatically by
/// every [`StyleAccess`] implementor that is also [`super::StateHost`];
/// builders without `StateHost` get every method except the state-slot
/// overlays, but do not satisfy this umbrella.
pub trait Styled:
    StyleAccess
    + Spacing
    + Sizing
    + Flex
    + Fill
    + Typography
    + Geometry
    + Visual
    + Transitions
    + StateVariants
{
}

impl<T> Styled for T where
    T: StyleAccess
        + Spacing
        + Sizing
        + Flex
        + Fill
        + Typography
        + Geometry
        + Visual
        + Transitions
        + StateVariants
{
}
