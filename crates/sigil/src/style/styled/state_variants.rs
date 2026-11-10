use super::StateStyleBuilder;
use crate::style::StateSlot;
use crate::widgets::{StateTarget, Widget};

/// Paint per interaction state, on builders that track hover, focus, press, and disabled.
pub trait States: Widget + StateTarget {
    /// Sets the style for the rest state.
    #[must_use]
    fn rest<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        self.set_state(
            StateSlot::Rest,
            configure(StateStyleBuilder::new()).into_atoms(),
        )
    }

    /// Sets the style while the pointer is over the node.
    #[must_use]
    fn hover<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        self.set_state(
            StateSlot::Hovered,
            configure(StateStyleBuilder::new()).into_atoms(),
        )
    }

    /// Sets the style while the node has focus.
    #[must_use]
    fn focus<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        self.set_state(
            StateSlot::Focused,
            configure(StateStyleBuilder::new()).into_atoms(),
        )
    }

    /// Sets the style while the node is pressed.
    #[must_use]
    fn pressed<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        self.set_state(
            StateSlot::Pressed,
            configure(StateStyleBuilder::new()).into_atoms(),
        )
    }

    /// Sets the style while the node is disabled.
    #[must_use]
    fn disabled_style<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        self.set_state(
            StateSlot::Disabled,
            configure(StateStyleBuilder::new()).into_atoms(),
        )
    }
}

impl<T: Widget + StateTarget> States for T {}
