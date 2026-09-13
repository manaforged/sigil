use std::rc::Rc;

use crate::events::prelude::{KeyEvent, PointerEvent};
use crate::kernel::{NodeBuilder, TextBuilder, TextContent, text};
use crate::reactive::{Signal, derived};
use crate::style::NodeStyleExt;
use crate::style::StyleState;

use crate::widgets::interaction::InteractionState;

pub(super) type Activate = Rc<dyn Fn() + 'static>;
pub(super) type BoolChange = Rc<dyn Fn(bool) + 'static>;
pub(super) type BuilderMutator = Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>;

#[must_use]
pub fn label(source: impl Into<TextContent>) -> TextBuilder {
    text(source).nowrap().class("text-on-surface")
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

pub(super) fn activate_pointer_reactive(
    event: &mut PointerEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    activate: &Activate,
) {
    if interaction.release_pointer(event) && !disabled_now(disabled) {
        activate();
    }
}

pub(super) fn activate_key_reactive(
    event: &mut KeyEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    activate: &Activate,
) {
    if interaction.activate_key(event) && !disabled_now(disabled) {
        activate();
    }
}

pub(crate) fn noop_activate() -> Activate {
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

pub(crate) fn binary_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let mut state = interaction.style_state();
    state.disabled |= disabled_now(disabled);
    state
}

pub(crate) fn reactive_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let _ = disabled.map(Signal::get);
    binary_style_state(interaction, disabled)
}

pub(crate) fn disabled_now(disabled: Option<Signal<bool>>) -> bool {
    disabled.is_some_and(Signal::peek)
}

pub(crate) fn or_disabled(
    a: Option<Signal<bool>>,
    b: Option<Signal<bool>>,
) -> Option<Signal<bool>> {
    match (a, b) {
        (Some(left), Some(right)) => Some(derived(move || left.get() || right.get())),
        (left, None) => left,
        (None, right) => right,
    }
}

pub(super) fn noop_bool_change() -> BoolChange {
    Rc::new(|_| {})
}

fn toggle_bool(value: Signal<bool>, on_change: &BoolChange) {
    let next = !value.peek();
    value.set(next);
    on_change(next);
}
