use std::rc::Rc;

use crate::events::{A11yEdit, with_handlers};
use crate::events::{Key, KeyEvent, LogicalKey, NodeEventsExt, PointerEvent};
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;

use crate::widgets::interaction::InteractionState;

use super::builder::{SliderRuntime, ValueChange};

pub(super) fn attach_key_handlers(
    mut node: NodeBuilder,
    interaction: InteractionState,
    runtime: SliderRuntime,
    on_change: ValueChange,
) -> NodeBuilder {
    for key in [
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowLeft,
        Key::ArrowDown,
        Key::PageUp,
        Key::PageDown,
        Key::Home,
        Key::End,
    ] {
        let change = Rc::clone(&on_change);
        node = node.key(key, move |event| {
            change_key(event, interaction, runtime, &change);
        });
    }
    with_handlers(node, move |handlers| {
        let edited = Rc::clone(&on_change);
        handlers.set_value_adjustment(Box::new(move |increase| {
            if !runtime.disabled() {
                runtime.nudge(if increase { 1.0 } else { -1.0 });
                on_change(runtime.value.peek());
            }
        }));
        handlers.set_a11y_edit(Box::new(move |edit| {
            let A11yEdit::Number(raw) = edit else {
                return false;
            };
            if !runtime.disabled() {
                runtime.value.set(runtime.quantize(*raw));
                edited(runtime.value.peek());
            }
            true
        }));
    })
}

pub(super) fn pointer_down(
    event: &mut PointerEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
) {
    if runtime.disabled() {
        return;
    }
    interaction.drag(true);
    event.capture_pointer();
    runtime.set_from_position(event);
}

pub(super) fn pointer_move(
    event: &mut PointerEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
) {
    if runtime.disabled() || !event.is_captured() || !interaction.dragging() {
        return;
    }
    runtime.set_from_position(event);
    event.stop_propagation();
}

pub(super) fn pointer_up(
    event: &mut PointerEvent,
    interaction: InteractionState,
    value: Signal<f32>,
    on_change: &ValueChange,
) {
    let was_dragging = interaction.dragging();
    interaction.drag(false);
    let _released = interaction.release_pointer(event);
    if was_dragging {
        on_change(value.peek());
    }
}

fn change_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    on_change: &ValueChange,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    let LogicalKey::Named(key) = &event.key else {
        return;
    };
    match key {
        Key::ArrowRight | Key::ArrowUp => runtime.nudge(1.0),
        Key::ArrowLeft | Key::ArrowDown => runtime.nudge(-1.0),
        Key::PageUp => runtime.nudge(10.0),
        Key::PageDown => runtime.nudge(-10.0),
        Key::Home => runtime.value.set(runtime.range.min),
        Key::End => runtime.value.set(runtime.range.max),
        _ => return,
    }
    on_change(runtime.value.peek());
}
