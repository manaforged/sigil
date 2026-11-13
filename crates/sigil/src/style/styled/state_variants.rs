use super::StateStyleBuilder;
use crate::widgets::Widget;

/// Paint per interaction state, on builders that track hover, focus, press, and disabled.
pub trait States: Widget {
    /// Sets the style while the node is not disabled, pressed, hovered, or focused.
    #[must_use]
    fn rest<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder;

    /// Sets the style while the pointer is over the node.
    #[must_use]
    fn hover<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder;

    /// Sets the style while the node has focus.
    #[must_use]
    fn focus<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder;

    /// Sets the style while the node is pressed.
    #[must_use]
    fn pressed<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder;

    /// Sets the style while the node is disabled.
    #[must_use]
    fn disabled_paint<F>(self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder;
}
