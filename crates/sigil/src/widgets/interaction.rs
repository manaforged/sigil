mod drag;
mod steps;

use crate::events::{KeyEvent, MouseButton, PointerEvent, PointerPhase, PointerType};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};
use crate::style::StyleState;
use crate::widgets::Widget;

pub(crate) use drag::{DragEnd, axis_of, pointer_drag};
pub(crate) use steps::{ValueStep, step_keys};

/// Hover, press, and focus signals.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InteractionState {
    disabled: Option<Signal<bool>>,
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    dragging: Signal<bool>,
    focused: Signal<bool>,
    ring_visible: Signal<bool>,
}

impl InteractionState {
    /// Creates an interaction state.
    #[must_use]
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            disabled: (!enabled).then(|| signal(true)),
            hovered: signal(false),
            pressed: signal(false),
            dragging: signal(false),
            focused: signal(false),
            ring_visible: signal(false),
        }
    }

    /// Makes the state follow `disabled` instead of the flag given to `new`.
    #[must_use]
    pub(crate) const fn disabled(mut self, disabled: Option<Signal<bool>>) -> Self {
        self.disabled = disabled;
        self
    }

    fn is_enabled(self) -> bool {
        !self.disabled.is_some_and(Signal::peek)
    }

    /// Returns the style state.
    #[must_use]
    pub(crate) fn style_state(self) -> StyleState {
        StyleState {
            disabled: self.disabled.is_some_and(Signal::get),
            hovered: self.hovered.get(),
            pressed: self.pressed.get() || self.dragging.get(),
            focused: self.focused.get(),
        }
    }

    /// Sets whether a drag is active.
    pub(crate) fn drag(self, next: bool) {
        set_if_changed(self.dragging, next);
    }

    /// Returns the signal that is true while the node has focus.
    #[must_use]
    pub(crate) const fn focused(self) -> Signal<bool> {
        self.focused
    }

    /// Attaches the state's handlers to `node`.
    #[must_use]
    pub(crate) fn install(self, node: NodeBuilder) -> NodeBuilder {
        let node = match self.disabled {
            Some(flag) => node.mutate(move |node| node.a11y.disabled = Some(flag)),
            None => node,
        };
        node.focusable(true)
            .hittable(true)
            .focus_ring(self.focus_ring())
            .focus_in(move || self.on_focus_in())
            .focus_out(move || self.on_focus_out())
            .pointer_enter(move |event| self.pointer_enter(event))
            .pointer_leave(move |event| self.pointer_leave(event))
            .pointer_down(move |event| self.pointer_down(event))
    }

    /// Returns the signal that shows the focus ring.
    #[must_use]
    pub(crate) const fn focus_ring(self) -> Signal<bool> {
        self.ring_visible
    }

    pub(crate) fn observe_style(self, node: NodeBuilder) -> NodeBuilder {
        node.focus_in(move || self.on_focus_in())
            .focus_out(move || self.on_focus_out())
            .pointer_enter(move |_| set_if_changed(self.hovered, true))
            .pointer_leave(move |_| self.leave())
            .pointer_down(move |event| {
                if is_primary(event) {
                    set_if_changed(self.pressed, true);
                }
            })
            .pointer_up(move |event| {
                if is_primary(event) || is_cancel(event) {
                    set_if_changed(self.pressed, false);
                }
            })
    }

    /// Handles a pointer release and returns whether it activates.
    pub(crate) fn release_pointer(self, event: &mut PointerEvent) -> bool {
        let cancelled = is_cancel(event);
        if !is_primary(event) && !cancelled {
            return false;
        }
        let was_pressed = self.pressed.peek();
        set_if_changed(self.pressed, false);
        let enabled = self.is_enabled();
        if enabled {
            event.stop_propagation();
        }
        enabled && !cancelled && was_pressed && self.hovered.peek()
    }

    /// Handles a key and returns whether it activates.
    pub(crate) fn activate_key(self, event: &mut KeyEvent) -> bool {
        let enabled = self.is_enabled();
        if enabled {
            event.stop_propagation();
        }
        enabled
    }

    fn pointer_enter(self, event: &mut PointerEvent) {
        set_if_changed(self.hovered, true);
        if self.is_enabled() {
            event.stop_propagation();
        }
    }

    fn pointer_leave(self, event: &mut PointerEvent) {
        self.leave();
        if self.is_enabled() {
            event.stop_propagation();
        }
    }

    fn leave(self) {
        set_if_changed(self.hovered, false);
        set_if_changed(self.pressed, false);
    }

    fn pointer_down(self, event: &mut PointerEvent) {
        if self.is_enabled() && is_primary(event) {
            set_if_changed(self.pressed, true);
            event.stop_propagation();
        }
    }

    fn on_focus_in(self) {
        set_if_changed(self.focused, true);
        set_if_changed(self.ring_visible, crate::events::focus_from_keyboard());
    }

    fn on_focus_out(self) {
        set_if_changed(self.focused, false);
        set_if_changed(self.ring_visible, false);
    }
}

pub(crate) fn is_primary(event: &PointerEvent) -> bool {
    event.button.map_or_else(
        || event.kind != PointerType::Mouse,
        |button| button == MouseButton::Left,
    )
}

pub(crate) fn is_cancel(event: &PointerEvent) -> bool {
    event.phase == PointerPhase::Cancel
}

fn set_if_changed<T: Clone + PartialEq + 'static>(signal: Signal<T>, next: T) {
    if signal.peek() != next {
        signal.set(next);
    }
}
