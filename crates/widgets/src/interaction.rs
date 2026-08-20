//! Reusable widget interaction contract.

use sigil_events::prelude::{KeyEvent, NodeEventsExt, PointerEvent};
use sigil_kernel::NodeBuilder;
use sigil_reactive::{Signal, signal};
use sigil_style::{FocusRing, NodeStyleExt, StyleState};

/// Shared hover/press/focus state for focusable widgets.
///
/// `focused` tracks logical focus (set on any focus, pointer or keyboard) and
/// drives state-styles + caret/display. `ring_visible` is the `:focus-visible`
/// signal: set only when focus arrives via the keyboard (or programmatically),
/// so a mouse click focuses a widget without raising the keyboard focus ring.
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
    /// Create a reusable interaction state bundle.
    ///
    /// # Panics
    ///
    /// The signals are owned by the current reactive scope. Calling
    /// outside [`Runtime::scope`](sigil_reactive::Runtime::scope)
    /// creates handles that panic on the next signal read.
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

    /// Whether this widget accepts interaction.
    #[must_use]
    pub const fn enabled(self) -> bool {
        self.enabled
    }

    /// Current state as a style resolver input.
    #[must_use]
    pub fn style_state(self) -> StyleState {
        StyleState {
            disabled: !self.enabled,
            hovered: self.hovered.get(),
            pressed: self.pressed.get() || self.dragging.get(),
            focused: self.focused.get(),
        }
    }

    /// Whether this widget is in an active drag gesture.
    #[must_use]
    pub fn dragging(self) -> bool {
        self.dragging.get()
    }

    /// Set whether this widget is in an active drag gesture.
    pub fn drag(self, next: bool) {
        set_if_changed(self.dragging, next);
    }

    /// The focus visibility signal used by focus-ring emission.
    #[must_use]
    pub const fn focus_visible(self) -> Signal<bool> {
        self.focused
    }

    /// Install the baseline widget handlers and focus-ring semantics.
    #[must_use]
    pub fn install(self, node: NodeBuilder) -> NodeBuilder {
        node.focusable(self.enabled)
            .hittable(self.enabled)
            .focus_ring(FocusRing::new(self.ring_visible))
            .focus_in(move || self.on_focus_in())
            .focus_out(move || self.on_focus_out())
            .pointer_enter(move |event| self.pointer_enter(event))
            .pointer_leave(move |event| self.pointer_leave(event))
            .pointer_down(move |event| self.pointer_down(event))
    }

    /// Release a pointer press and return whether this was a valid activation.
    pub fn release_pointer(self, event: &mut PointerEvent) -> bool {
        let was_pressed = self.pressed.peek();
        self.set_pressed(false);
        if self.enabled {
            event.stop_propagation();
        }
        self.enabled && was_pressed && self.hovered.peek()
    }

    /// Return whether a key event should activate this widget.
    #[allow(clippy::missing_const_for_fn)]
    pub fn activate_key(self, event: &mut KeyEvent) -> bool {
        if self.enabled {
            event.stop_propagation();
        }
        self.enabled
    }

    fn pointer_enter(self, event: &mut PointerEvent) {
        if self.enabled {
            self.set_hovered(true);
            event.stop_propagation();
        }
    }

    fn pointer_leave(self, event: &mut PointerEvent) {
        if self.enabled {
            self.set_hovered(false);
            self.set_pressed(false);
            event.stop_propagation();
        }
    }

    fn pointer_down(self, event: &mut PointerEvent) {
        if self.enabled {
            self.set_pressed(true);
            event.stop_propagation();
        }
    }

    fn set_hovered(self, next: bool) {
        set_if_changed(self.hovered, next);
    }

    fn set_pressed(self, next: bool) {
        set_if_changed(self.pressed, next);
    }

    fn set_focused(self, next: bool) {
        set_if_changed(self.focused, next);
    }

    /// On gaining focus: always logically focused (drives state-style + caret),
    /// but only raise the focus-visible ring when focus came from the keyboard
    /// (or programmatically) — not from a mouse click.
    fn on_focus_in(self) {
        self.set_focused(true);
        set_if_changed(self.ring_visible, sigil_events::focus_from_keyboard());
    }

    fn on_focus_out(self) {
        self.set_focused(false);
        set_if_changed(self.ring_visible, false);
    }
}

fn set_if_changed<T>(signal: Signal<T>, next: T)
where
    T: Clone + PartialEq + 'static,
{
    if signal.peek() != next {
        signal.set(next);
    }
}
