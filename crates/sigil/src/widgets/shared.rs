use std::rc::Rc;

use sigil_events::prelude::{KeyEvent, PointerEvent};
use sigil_kernel::{NodeBuilder, TextBuilder, TextContent, text};
use sigil_reactive::Signal;
use sigil_style::NodeStyleExt;
use sigil_style::StyleState;

use crate::interaction::InteractionState;

pub(super) type Activate = Rc<dyn Fn() + 'static>;
pub(super) type BoolChange = Rc<dyn Fn(bool) + 'static>;
pub(super) type BuilderMutator = Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>;

/// Create a text label with the default readable surface colour.
#[must_use]
pub fn label(source: impl Into<TextContent>) -> TextBuilder {
    text(source).class("text-on-surface")
}

pub(super) fn activate_pointer(
    event: &mut PointerEvent,
    interaction: InteractionState,
    activate: &Activate,
) {
    if interaction.release_pointer(event) {
        activate();
    }
}

pub(super) fn activate_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    activate: &Activate,
) {
    if interaction.activate_key(event) {
        activate();
    }
}

pub(super) fn noop_activate() -> Activate {
    Rc::new(|| {})
}

pub(super) fn activate_binary_pointer(
    event: &mut PointerEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    value: Signal<bool>,
    on_change: &BoolChange,
) {
    let valid_release = interaction.release_pointer(event);
    if valid_release && !disabled_now(disabled) {
        toggle_bool(value, on_change);
    }
}

pub(super) fn activate_binary_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    value: Signal<bool>,
    on_change: &BoolChange,
) {
    if interaction.activate_key(event) && !disabled_now(disabled) {
        toggle_bool(value, on_change);
    }
}

pub(super) fn binary_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let mut state = interaction.style_state();
    state.disabled |= disabled_now(disabled);
    state
}

pub(super) fn disabled_now(disabled: Option<Signal<bool>>) -> bool {
    disabled.is_some_and(Signal::peek)
}

pub(super) fn noop_bool_change() -> BoolChange {
    Rc::new(|_| {})
}

fn toggle_bool(value: Signal<bool>, on_change: &BoolChange) {
    let next = !value.peek();
    value.set(next);
    on_change(next);
}
