use std::cell::Cell;
use std::rc::Rc;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::NodeBuilder;
use sigil_reactive::{Signal, effect, signal};
use sigil_style::{ClassSource, NodeStyleExt, ReactiveStateSlot, StateSlot, Style, StyleAccess};

pub struct InteractiveBuilder {
    inner: NodeBuilder,
    style: Style,
    disabled: Option<Signal<bool>>,
    focusable: bool,
}

#[must_use]
pub fn interactive(inner: NodeBuilder) -> InteractiveBuilder {
    InteractiveBuilder {
        inner,
        style: Style::new(),
        disabled: None,
        focusable: false,
    }
}

impl InteractiveBuilder {
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.focusable = yes;
        self
    }

    #[must_use]
    pub const fn disabled_signal(mut self, signal: Signal<bool>) -> Self {
        self.disabled = Some(signal);
        self
    }

    #[must_use]
    pub fn class(mut self, classes: &str) -> Self {
        self.style = core::mem::take(&mut self.style).class(classes);
        self
    }

    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let mut style = self.style;
        let overlay = style.take_state_overlay();
        let base_atoms = style.into_atoms();

        let hovered = signal(false);
        let pressed = signal(false);
        let focused = signal(false);
        let disabled = self.disabled;

        let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
        publish_slot_on_change(&slot_cell, hovered, pressed, focused, disabled);

        let mut node = self.inner.focusable(self.focusable);
        if !base_atoms.is_empty() {
            node = node.class(ClassSource::Owned(Box::new(base_atoms)));
        }
        if !overlay.is_empty() {
            node = node.state_overlay(overlay, slot_cell);
        }
        node.pointer_enter(move |_event| {
            if !disabled_is(disabled.as_ref()) {
                set_if_changed(hovered, true);
            }
        })
        .pointer_leave(move |_event| {
            set_if_changed(hovered, false);
            set_if_changed(pressed, false);
        })
        .pointer_down(move |_event| {
            if !disabled_is(disabled.as_ref()) {
                set_if_changed(pressed, true);
            }
        })
        .pointer_up(move |_event| {
            set_if_changed(pressed, false);
        })
        .focus_in(move || {
            if !disabled_is(disabled.as_ref()) {
                set_if_changed(focused, true);
            }
        })
        .focus_out(move || {
            set_if_changed(focused, false);
        })
    }
}

impl StyleAccess for InteractiveBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl sigil_style::StateHost for InteractiveBuilder {}

impl sigil_kernel::IntoNode for InteractiveBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn publish_slot_on_change(
    slot_cell: &ReactiveStateSlot,
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    focused: Signal<bool>,
    disabled: Option<Signal<bool>>,
) {
    let cell = Rc::clone(slot_cell);
    effect(move || {
        let slot = if disabled_is(disabled.as_ref()) {
            StateSlot::Disabled
        } else if pressed.get() {
            StateSlot::Pressed
        } else if hovered.get() {
            StateSlot::Hovered
        } else if focused.get() {
            StateSlot::Focused
        } else {
            StateSlot::Rest
        };
        cell.set(slot);
    });
}

fn disabled_is(disabled: Option<&Signal<bool>>) -> bool {
    disabled.copied().is_some_and(Signal::get)
}

fn set_if_changed(signal: Signal<bool>, next: bool) {
    if signal.peek() != next {
        signal.set(next);
    }
}
