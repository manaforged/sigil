//! Theme shape language — orthogonal to colour [`Palette`](crate::palette::Palette).
//!
//! A `Variant` is the "shape vocabulary" of a theme: font family,
//! corner radius, shadow strength, whether hover state lifts. Any
//! palette can be combined with any variant (Mythril+Terminal is a
//! perfectly valid theme), so the two are kept separate by design.
//!
//! Two variants ship today:
//!
//! - [`Variant::Normal`] — proportional font, gentle radius, subtle
//!   shadows, hover state lifts. The default.
//! - [`Variant::Terminal`] — monospace font, zero radius, no shadows,
//!   flat hover. The runic-shell stress-test aesthetic and the second
//!   locked regression baseline.
//!
//! Future variants (Brutalist, Glass, Soft) are additive — adding one
//! does not touch palettes.

/// Shape language identifier. Pair with a [`Palette`](crate::palette::Palette)
/// to produce a [`Theme`](crate::Theme).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Proportional font, gentle radius, subtle shadows, hover-lift.
    /// The default for shipped desktop and game UIs.
    #[default]
    Normal,
    /// Monospace font, zero radius, no shadows, flat hover. Locked
    /// regression baseline for [`Theme::terminal`](crate::Theme).
    Terminal,
}

/// Hint to text shapers: the variant's preferred font family.
///
/// Two coarse buckets ship today — proportional and monospace.
/// Variants that need a serif face, a specific weight axis, or a
/// per-language fallback chain must compose those choices at the
/// renderer layer; this hint is the variant's single contribution to
/// font selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFamilyHint {
    /// Whatever the renderer treats as its default proportional UI
    /// font.
    #[default]
    Proportional,
    /// Whatever the renderer treats as its default monospace font.
    Monospace,
}

/// Concrete shape tokens derived from a [`Variant`]. Themes carry one
/// of these alongside their palette.
///
/// `shadows` and `hover_lift` are independent flags. A future
/// Brutalist variant could pair `shadows = false` with
/// `hover_lift = true` for sharp edges that still respond to pointer
/// state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariantTokens {
    /// Corner radius applied to surfaces, in logical pixels.
    pub radius: u16,
    /// Whether the variant draws drop shadows under elevated surfaces.
    pub shadows: bool,
    /// Whether hovered surfaces brighten/darken versus their base
    /// colour. [`crate::Theme::from_palette`] consults this when
    /// deriving `accent_hover` and `surface_hover`.
    pub hover_lift: bool,
    /// Font-family hint passed to the text engine.
    pub font: FontFamilyHint,
}

impl Variant {
    /// The concrete shape tokens for this variant.
    #[must_use]
    pub const fn tokens(self) -> VariantTokens {
        match self {
            Self::Normal => VariantTokens {
                radius: 8,
                shadows: true,
                hover_lift: true,
                font: FontFamilyHint::Proportional,
            },
            Self::Terminal => VariantTokens {
                radius: 0,
                shadows: false,
                hover_lift: false,
                font: FontFamilyHint::Monospace,
            },
        }
    }
}

#[cfg(test)]
#[path = "variant_tests.rs"]
mod tests;
