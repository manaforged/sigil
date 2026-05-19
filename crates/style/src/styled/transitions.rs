//! `Transitions` super-trait — deferred transition preset and duration.

use super::StyleAccess;
use crate::{MillisU16, StyleAtom, TransitionId};

/// Transition methods on the fluent styling surface.
pub trait Transitions: StyleAccess {
    /// Store a deferred transition preset (animation runtime lands in
    /// W4.G9; G6 stores the intent only).
    #[must_use]
    fn transition(mut self, id: TransitionId) -> Self {
        self.style().push_atom(StyleAtom::Transition(id));
        self
    }

    /// Store a deferred transition duration.
    #[must_use]
    fn duration(mut self, duration: MillisU16) -> Self {
        self.style()
            .push_atom(StyleAtom::TransitionDuration(duration));
        self
    }
}

impl<T: StyleAccess> Transitions for T {}
