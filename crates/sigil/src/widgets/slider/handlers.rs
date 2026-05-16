use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt, PointerEvent};
use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

use crate::interaction::InteractionState;

use super::builder::{SliderRange, SliderRuntime, ValueChange};

pub(super) fn attach_key_handlers(
    node: NodeBuilder,
    interaction: InteractionState,
    runtime: SliderRuntime,
) -> NodeBuilder {
    node.on_key(Key::ArrowRight, move |event| {
        step_key(event, interaction, runtime, true);
    })
    .on_key(Key::ArrowUp, move |event| {
        step_key(event, interaction, runtime, true);
    })
    .on_key(Key::ArrowLeft, move |event| {
        step_key(event, interaction, runtime, false);
    })
    .on_key(Key::ArrowDown, move |event| {
        step_key(event, interaction, runtime, false);
    })
    .on_key(Key::PageUp, move |event| {
        page_key(event, interaction, runtime, true);
    })
    .on_key(Key::PageDown, move |event| {
        page_key(event, interaction, runtime, false);
    })
    .on_key(Key::Home, move |event| {
        jump_key(event, interaction, runtime, false);
    })
    .on_key(Key::End, move |event| {
        jump_key(event, interaction, runtime, true);
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
    interaction.set_dragging(true);
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
    interaction.set_dragging(false);
    let _released = interaction.release_pointer(event);
    if was_dragging {
        on_change(value.peek());
    }
}

fn step_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    increase: bool,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    runtime.step_value(increase);
}

fn page_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    increase: bool,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    runtime.page_step_value(increase);
}

fn jump_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    max: bool,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    runtime.jump_to_end(max);
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
