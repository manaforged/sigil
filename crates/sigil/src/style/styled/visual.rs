use super::StyleAccess;
use crate::style::{OverflowKind, StyleAtom, Transform2D};

const OPACITY_SCALE: f32 = 10_000.0;

pub trait Visual: StyleAccess {
    #[must_use]
    fn transform(mut self, transform: Transform2D) -> Self {
        self.style_mut().push_atom(StyleAtom::Transform(transform));
        self
    }

    #[must_use]
    #[track_caller]
    fn opacity(mut self, value: f32) -> Self {
        debug_assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "opacity must be a finite value in 0.0..=1.0, got {value}",
        );
        let clamped = value.clamp(0.0, 1.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "values are bounded to the target range before the cast"
        )]
        let encoded = (clamped * OPACITY_SCALE).round() as u16;
        self.style_mut().push_atom(StyleAtom::Opacity(encoded));
        self
    }

    #[must_use]
    fn overflow_x(mut self, kind: OverflowKind) -> Self {
        self.style_mut().push_atom(StyleAtom::OverflowX(kind));
        self
    }

    #[must_use]
    fn overflow_y(mut self, kind: OverflowKind) -> Self {
        self.style_mut().push_atom(StyleAtom::OverflowY(kind));
        self
    }

    #[must_use]
    fn overflow_hidden(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::OverflowX(OverflowKind::Hidden));
        self.style_mut()
            .push_atom(StyleAtom::OverflowY(OverflowKind::Hidden));
        self
    }
}

impl<T: StyleAccess> Visual for T {}
