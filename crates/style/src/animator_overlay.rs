//! Animator-overlay seam consumed by [`StyleEngine::apply_with_overlay`].
//!
//! W5.1b — DR-053 §5 phase 1. The seam is a trait the engine calls when
//! a [`StyleBinding`](crate::StyleBinding) has an attached animator set.
//! Implementations live in `sigil-animation-runtime` so this crate does
//! not depend on the animator runtime.
//!
//! The trait surface is read-only at apply time: the engine asks "is
//! there a driven value for this numeric atom kind?" and substitutes
//! the result for the literal atom value. Retargeting on state change
//! goes through [`AnimatorOverlay::retarget_to_slot`], which the
//! reactive effect calls when the interaction state slot changes.

use crate::{Rgba8, StateSlot, StyleAtom};

/// Numeric style atoms the W5.1 animator hook recognises as animatable.
///
/// W5.1c shipped `Opacity` + `BorderRadius` engine dispatch; the other
/// kinds (font/line-height/gap/padding) are recognised by
/// `install_animators` but only the render-visible kinds are read by
/// `StyleEngine::apply_with_overlay`. Layout-flowing kinds will be
/// reached by the layout-side overlay seam in a follow-up phase.
/// `sigil-animation-runtime` re-exports this enum for ergonomics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericAtomKind {
    /// `StyleAtom::Opacity(u16)` — encoded as `value * 10_000`.
    Opacity,
    /// `StyleAtom::BorderRadius(Px)`.
    BorderRadius,
    /// `StyleAtom::FontSize(Px)`.
    FontSize,
    /// `StyleAtom::LineHeight(Px)`.
    LineHeight,
    /// `StyleAtom::Gap(Px)`.
    Gap,
    /// `StyleAtom::PaddingX(Px)`.
    PaddingX,
    /// `StyleAtom::PaddingY(Px)`.
    PaddingY,
}

/// Colour-bearing style atoms the W5.1c animator hook recognises as
/// animatable.
///
/// `*ThemeRef` variants share kinds with their `*Color` siblings — the
/// theme reference is resolved to an `Rgba8` at install time, so the
/// animator interpolates the resolved colour rather than the token name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorAtomKind {
    /// `StyleAtom::BackgroundColor` / `StyleAtom::BackgroundThemeRef`.
    Background,
    /// `StyleAtom::TextColor` / `StyleAtom::TextThemeRef`.
    Text,
}

/// Read-only animator overlay consulted by the style engine.
///
/// Implementations expose the *current* driven value for each
/// recognised numeric atom kind. The engine reads `value_for(kind)`
/// during apply; if `Some(v)`, the resolved render style uses `v`
/// instead of the literal atom value.
///
/// `retarget_to_slot` is called by the binding's reactive effect when
/// the widget's interaction state slot changes — the implementation
/// re-aims its underlying animators at the new slot's atom values,
/// driving the smooth transition.
pub trait AnimatorOverlay {
    /// Return the current driven value for `kind`, or `None` when no
    /// animator drives that kind.
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32>;

    /// Return the current driven colour for `kind`, or `None` when no
    /// animator drives that kind. W5.1c — colour interpolation in
    /// linear-light space.
    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8>;

    /// Re-aim every animator at the state-slot's value, using the
    /// owning binding's base atoms + state-style overlay tables. The
    /// implementation knows the state-style tables it was built from.
    fn retarget_to_slot(&mut self, slot: StateSlot, base: &[StyleAtom]);
}
