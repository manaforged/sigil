//! `Visual` super-trait — opacity and overflow behaviour.

use super::StyleAccess;
use crate::{OverflowKind, StyleAtom};

/// Internal full-scale value the `StyleAtom::Opacity` u16 representation
/// uses. `0.0 → 0`, `1.0 → 10_000`.
const OPACITY_SCALE: f32 = 10_000.0;

/// Visual-axis methods on the fluent styling surface — opacity and
/// overflow.
pub trait Visual: StyleAccess {
    /// Set opacity in the CSS-canonical `0.0..=1.0` range (`0.0` =
    /// fully transparent, `1.0` = fully opaque). The internal
    /// `StyleAtom::Opacity` representation scales by `10_000`, but
    /// authoring uses the float convention; values outside the range
    /// are clamped and trigger a `debug_assert!` so misuse is caught
    /// in debug builds.
    #[must_use]
    #[track_caller]
    fn opacity(mut self, value: f32) -> Self {
        debug_assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "opacity must be a finite value in 0.0..=1.0, got {value}",
        );
        let clamped = value.clamp(0.0, 1.0);
        // SAFETY (lint): `clamped` is bounded `0.0..=1.0`, scale is 10_000;
        // product is in `0.0..=10_000.0`, which fits losslessly in `u16`
        // (max 65_535) and is non-negative. No truncation or sign loss.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let encoded = (clamped * OPACITY_SCALE).round() as u16;
        self.style().push_atom(StyleAtom::Opacity(encoded));
        self
    }

    /// Set inline-axis overflow behaviour.
    #[must_use]
    fn overflow_x(mut self, kind: OverflowKind) -> Self {
        self.style().push_atom(StyleAtom::OverflowX(kind));
        self
    }

    /// Set block-axis overflow behaviour.
    #[must_use]
    fn overflow_y(mut self, kind: OverflowKind) -> Self {
        self.style().push_atom(StyleAtom::OverflowY(kind));
        self
    }

    /// Hide overflow on both axes.
    #[must_use]
    fn overflow_hidden(mut self) -> Self {
        self.style()
            .push_atom(StyleAtom::OverflowX(OverflowKind::Hidden));
        self.style()
            .push_atom(StyleAtom::OverflowY(OverflowKind::Hidden));
        self
    }
}

impl<T: StyleAccess> Visual for T {}
