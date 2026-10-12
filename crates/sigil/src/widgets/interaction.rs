use crate::events::{KeyEvent, MouseButton, NodeEventsExt, PointerEvent};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};
use crate::style::{FocusRing, NodeStyleExt, StyleState};

#[derive(Clone, Copy)]
pub struct InteractionState {
    enabled: bool,
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    dragging: Signal<bool>,
    focused: Signal<bool>,
    ring_visible: Signal<bool>,
}

impl InteractionState {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            hovered: signal(false),
            pressed: signal(false),
            dragging: signal(false),
            focused: signal(false),
            ring_visible: signal(false),
        }
    }

    #[must_use]
    pub const fn enabled(self) -> bool {
        self.enabled
    }

    #[must_use]
    pub fn style_state(self) -> StyleState {
        StyleState {
            disabled: !self.enabled,
            hovered: self.hovered.get(),
            pressed: self.pressed.get() || self.dragging.get(),
            focused: self.focused.get(),
        }
    }

    #[must_use]
    pub fn dragging(self) -> bool {
        self.dragging.get()
    }

    pub fn drag(self, next: bool) {
        set_if_changed(self.dragging, next);
    }

    #[must_use]
    pub const fn focus_visible(self) -> Signal<bool> {
        self.focused
    }

    #[must_use]
    pub fn install(self, node: NodeBuilder) -> NodeBuilder {
        node.focusable(self.enabled)
            .hittable(self.enabled)
            .focus_ring(self.focus_ring())
            .focus_in(move || self.on_focus_in())
            .focus_out(move || self.on_focus_out())
            .pointer_enter(move |event| self.pointer_enter(event))
            .pointer_leave(move |event| self.pointer_leave(event))
            .pointer_down(move |event| self.pointer_down(event))
    }

    #[must_use]
    pub const fn focus_ring(self) -> FocusRing {
        FocusRing::new(self.ring_visible)
    }

    pub(crate) fn observe_style(self, node: NodeBuilder) -> NodeBuilder {
        node.focus_in(move || self.on_focus_in())
            .focus_out(move || self.on_focus_out())
            .pointer_enter(move |_| set_if_changed(self.hovered, true))
            .pointer_leave(move |_| self.leave())
            .pointer_down(move |event| {
                if event.button == Some(MouseButton::Left) {
                    set_if_changed(self.pressed, true);
                }
            })
            .pointer_up(move |event| {
                if event.button == Some(MouseButton::Left) {
                    set_if_changed(self.pressed, false);
                }
            })
    }

    pub fn release_pointer(self, event: &mut PointerEvent) -> bool {
        if event.button != Some(MouseButton::Left) {
            return false;
        }
        let was_pressed = self.pressed.peek();
        set_if_changed(self.pressed, false);
        if self.enabled {
            event.stop_propagation();
        }
        self.enabled && was_pressed && self.hovered.peek()
    }

    #[allow(
        clippy::missing_const_for_fn,
        reason = "const-eligible only under one cfg configuration"
    )]
    pub fn activate_key(self, event: &mut KeyEvent) -> bool {
        if self.enabled {
            event.stop_propagation();
        }
        self.enabled
    }

    fn pointer_enter(self, event: &mut PointerEvent) {
        if self.enabled {
            set_if_changed(self.hovered, true);
            event.stop_propagation();
        }
    }

    fn pointer_leave(self, event: &mut PointerEvent) {
        if self.enabled {
            self.leave();
            event.stop_propagation();
        }
    }

    fn leave(self) {
        set_if_changed(self.hovered, false);
        set_if_changed(self.pressed, false);
    }

    fn pointer_down(self, event: &mut PointerEvent) {
        if self.enabled && event.button == Some(MouseButton::Left) {
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

fn set_if_changed<T: Clone + PartialEq + 'static>(signal: Signal<T>, next: T) {
    if signal.peek() != next {
        signal.set(next);
    }
}
