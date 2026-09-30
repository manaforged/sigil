use smallvec::SmallVec;

use super::StateStyleBuilder;
use crate::style::{PaintTarget, StateHost, Style, StyleAtom};

type Setter = fn(&mut Style, SmallVec<[StyleAtom; 16]>);

fn set_state<T, F>(mut target: T, set: Setter, configure: F) -> T
where
    T: PaintTarget,
    F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
{
    let built = configure(StateStyleBuilder::new());
    set(target.style_slot().0, built.into_atoms());
    target
}

pub trait StateVariants: PaintTarget + StateHost {
    #[must_use]
    fn rest<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, Style::set_rest, configure)
    }

    #[must_use]
    fn hover<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, Style::set_hover, configure)
    }

    #[must_use]
    fn focus<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, Style::set_focus, configure)
    }

    #[must_use]
    fn pressed<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, Style::set_pressed, configure)
    }

    #[must_use]
    fn disabled_style<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, Style::set_disabled, configure)
    }
}

impl<T: PaintTarget + StateHost> StateVariants for T {}
