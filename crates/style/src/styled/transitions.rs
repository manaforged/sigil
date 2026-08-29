use super::StyleAccess;
use crate::{MillisU16, StyleAtom, TransitionId};

pub trait Transitions: StyleAccess {
    #[must_use]
    fn transition(mut self, id: TransitionId) -> Self {
        self.style_mut().push_atom(StyleAtom::Transition(id));
        self
    }

    #[must_use]
    fn duration(mut self, duration: MillisU16) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::TransitionDuration(duration));
        self
    }
}

impl<T: StyleAccess> Transitions for T {}
