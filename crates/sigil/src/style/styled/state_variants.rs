use super::StateStyleBuilder;
use crate::style::{PaintTarget, StateHost, StateSlot};

fn set_state<T, F>(mut target: T, slot: StateSlot, configure: F) -> T
where
    T: PaintTarget,
    F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
{
    let built = configure(StateStyleBuilder::new());
    target.style_slot().0.set_state(slot, built.into_atoms());
    target
}

pub trait StateVariants: PaintTarget + StateHost {
    #[must_use]
    fn rest<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, StateSlot::Rest, configure)
    }

    #[must_use]
    fn hover<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, StateSlot::Hovered, configure)
    }

    #[must_use]
    fn focus<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, StateSlot::Focused, configure)
    }

    #[must_use]
    fn pressed<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, StateSlot::Pressed, configure)
    }

    #[must_use]
    fn disabled_style<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        set_state(self, StateSlot::Disabled, configure)
    }
}

impl<T: PaintTarget + StateHost> StateVariants for T {}
