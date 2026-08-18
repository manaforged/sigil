//! Semantic floater intent → theme-token color mapping.
//!
//! [`FloaterStyle`] keeps damage-floater callers off ad-hoc RGBA and on
//! the same [`sigil_style::scale::ColorScale`] steps the rest of the design
//! system draws from.

use sigil_style::scale::ScaleRole;
use sigil_style::{Rgba8, Theme};

/// Semantic damage-floater intent, set via [`super::FloatingTextBuilder::style`].
///
/// Overrides every spawned particle's color with the resolved theme
/// token, in place of whatever [`super::FctSpawn::color`] the caller
/// supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloaterStyle {
    /// Damage taken — resolves to the error scale.
    Damage,
    /// Healing received — resolves to the success scale.
    Heal,
    /// Attack missed — resolves to the neutral gray scale.
    Miss,
}

impl FloaterStyle {
    /// `(scale role, step)` pair. Mirrors `Tone`'s foreground
    /// convention (`text-<role>-11`, `text-gray-12` for neutral — see
    /// `chip.rs`) so floater colors read consistently with the rest of
    /// the design system's status text.
    const fn scale_step(self) -> (ScaleRole, u8) {
        match self {
            Self::Damage => (ScaleRole::Error, 11),
            Self::Heal => (ScaleRole::Success, 11),
            Self::Miss => (ScaleRole::Gray, 12),
        }
    }

    /// Resolve to a literal [`Rgba8`] against the default theme.
    ///
    /// # Hygiene-exception — literal color, not a `StyleAtom`
    ///
    /// Floating-text particles bypass the node-based style engine (the
    /// pool is read directly by a downstream renderer without invoking
    /// layout — see the crate module doc), so there is no per-node
    /// style atom to attach a theme reference to. This resolves eagerly
    /// against [`Theme::default`] instead, reusing the same
    /// `ColorScale` steps every themed widget draws from — not a new
    /// hardcoded hex value.
    #[must_use]
    pub fn color(self) -> Rgba8 {
        let (role, step) = self.scale_step();
        Theme::default().scale(role).step(step)
    }
}
