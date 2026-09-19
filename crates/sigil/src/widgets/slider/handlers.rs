use std::rc::Rc;

use crate::events::prelude::{Key, KeyEvent, LogicalKey, NodeEventsExt, PointerEvent};
use crate::events::with_handlers;
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;

use crate::widgets::interaction::InteractionState;

use super::builder::{SliderRange, SliderRuntime, ValueChange};

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
        handlers.set_value_adjustment(Box::new(move |increase| {
            if !runtime.disabled() {
                runtime.step_value(increase);
                on_change(runtime.value.peek());
            }
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
    match event.key {
        LogicalKey::Named(Key::ArrowRight | Key::ArrowUp) => runtime.step_value(true),
        LogicalKey::Named(Key::ArrowLeft | Key::ArrowDown) => runtime.step_value(false),
        LogicalKey::Named(Key::PageUp) => runtime.page_step_value(true),
        LogicalKey::Named(Key::PageDown) => runtime.page_step_value(false),
        LogicalKey::Named(Key::Home) => runtime.jump_to_end(false),
        LogicalKey::Named(Key::End) => runtime.jump_to_end(true),
        _ => return,
    }
    on_change(runtime.value.peek());
}

pub(super) fn quantize(range: SliderRange, raw: f32, step: f32) -> f32 {
    let clamped = range.clamp(raw);
    let step = normalized_step(step);
    let steps = ((clamped - range.min) / step).round();
    range.clamp(steps.mul_add(step, range.min))
}

pub(super) fn normalized_step(step: f32) -> f32 {
    if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    }
}
