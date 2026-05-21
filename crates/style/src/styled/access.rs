//! `StyleAccess` — the single mutable-borrow hook every styling
//! super-trait builds on.
//!
//! Implementors provide [`StyleAccess::style_mut`]; the per-category
//! super-traits ([`super::Spacing`], [`super::Sizing`], etc.) attach
//! their default methods on top via blanket impls over `StyleAccess`.
//! See DR-049 for the locked vocabulary.

use crate::Style;

/// Mutable borrow of a builder's owned [`Style`] accumulator.
///
/// This is the single method every styling implementor must provide.
/// Every per-category super-trait ([`super::Spacing`], [`super::Sizing`],
/// [`super::Flex`], [`super::Fill`], [`super::Typography`],
/// [`super::Geometry`], [`super::Visual`], [`super::Transitions`],
/// [`super::StateVariants`]) is a blanket impl over `StyleAccess`, so
/// implementing this trait is enough to get the full fluent surface.
pub trait StyleAccess: Sized {
    /// Mutable borrow of this builder's owned [`Style`] accumulator.
    fn style_mut(&mut self) -> &mut Style;
}
