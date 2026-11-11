use super::StateStyleBuilder;
use crate::style::StateSlot;
use crate::widgets::{StatePaint, StateTarget, Widget};

/// Paint per interaction state, on builders that track hover, focus, press, and disabled.
pub trait States: Widget + StateTarget {
    /// Sets the style for the rest state.
    #[must_use]
    fn rest<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        state(self, StateSlot::Rest, configure)
    }

    /// Sets the style while the pointer is over the node.
    #[must_use]
    fn hover<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        state(self, StateSlot::Hovered, configure)
    }

    /// Sets the style while the node has focus.
    #[must_use]
    fn focus<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        state(self, StateSlot::Focused, configure)
    }

    /// Sets the style while the node is pressed.
    #[must_use]
    fn pressed<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        state(self, StateSlot::Pressed, configure)
    }

    /// Sets the style while the node is disabled.
    #[must_use]
    fn disabled_style<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        state(self, StateSlot::Disabled, configure)
    }
}

impl<T: Widget + StateTarget> States for T {}

fn state<W, F>(widget: W, slot: StateSlot, configure: F) -> W
where
    W: StateTarget,
    F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
{
    widget.set_state(StatePaint {
        slot,
        atoms: configure(StateStyleBuilder::new()).into_atoms(),
    })
}
