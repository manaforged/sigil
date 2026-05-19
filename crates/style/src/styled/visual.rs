//! `Visual` super-trait — opacity and overflow behaviour.

use super::StyleAccess;
use crate::{OverflowKind, StyleAtom};

/// Visual-axis methods on the fluent styling surface — opacity and
/// overflow.
pub trait Visual: StyleAccess {
    /// Set opacity (encoded as `value * 10000`; e.g. `5_000` = 50 %).
    #[must_use]
    fn opacity(mut self, value: u16) -> Self {
        self.style().push_atom(StyleAtom::Opacity(value));
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
