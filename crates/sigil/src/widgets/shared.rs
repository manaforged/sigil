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

/// Pointer-release activation that also respects a reactive disabled signal.
///
/// Suppresses the callback when `disabled` peeks `true`, even if the
/// initial constructor enabled state allowed interaction.
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

/// Key-activation that also respects a reactive disabled signal.
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

/// A no-op activation callback — the default for interactive widgets
/// that haven't been wired with `.activate(...)` yet.
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

/// Fold the widget's interaction state with the reactive `disabled`
/// signal into a [`StyleState`]. Peeks the signal — use
/// [`reactive_style_state`] when the enclosing reactive closure
/// should re-run on `disabled` flips.
pub(crate) fn binary_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let mut state = interaction.style_state();
    state.disabled |= disabled_now(disabled);
    state
}

/// Same as [`binary_style_state`] but also subscribes to `disabled` so
/// the enclosing reactive effect re-runs on signal flips.
///
/// Use this inside a `stateful_style` closure when the widget has a
/// reactive `disabled` signal; the standalone `binary_style_state`
/// only peeks and won't drive re-resolution on its own.
pub(crate) fn reactive_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let _ = disabled.map(Signal::get);
    binary_style_state(interaction, disabled)
}

/// Peek a reactive disabled signal — `false` when absent.
pub(crate) fn disabled_now(disabled: Option<Signal<bool>>) -> bool {
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
