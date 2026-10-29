use std::rc::Rc;

use crate::events::{A11yEdit, with_handlers};
use crate::events::{Key, KeyEvent};
use crate::kernel::NodeBuilder;

use crate::widgets::interaction::{InteractionState, Step, pointer_drag, step_keys};

use super::builder::{SliderRuntime, ValueChange};

const SLIDER_KEYS: [(Key, Step); 8] = [
    (Key::ArrowRight, Step::Unit(1.0)),
    (Key::ArrowUp, Step::Unit(1.0)),
    (Key::ArrowLeft, Step::Unit(-1.0)),
    (Key::ArrowDown, Step::Unit(-1.0)),
    (Key::PageUp, Step::Page(1.0)),
    (Key::PageDown, Step::Page(-1.0)),
    (Key::Home, Step::Start),
    (Key::End, Step::End),
];

const PAGE_STEPS: f32 = 10.0;

pub(super) fn attach_key_handlers(
    node: NodeBuilder,
    interaction: InteractionState,
    runtime: SliderRuntime,
    on_change: ValueChange,
) -> NodeBuilder {
    let change = Rc::clone(&on_change);
    let node = step_keys(node, &SLIDER_KEYS, move |step, event| {
        change_key(step, event, interaction, runtime, &change);
    });
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

pub(super) fn attach_pointer_drag(
    node: NodeBuilder,
    interaction: InteractionState,
    runtime: SliderRuntime,
    on_change: ValueChange,
) -> NodeBuilder {
    pointer_drag(
        node,
        move |event| {
            if runtime.disabled() {
                return None;
            }
            interaction.drag(true);
            runtime.set_from_position(event);
            Some(())
        },
        move |(), event| {
            if !runtime.disabled() {
                runtime.set_from_position(event);
                event.stop_propagation();
            }
        },
        move |ended, event| {
            interaction.drag(false);
            let _released = interaction.release_pointer(event);
            if ended.is_some() {
                on_change(runtime.value.peek());
            }
        },
    )
}

fn change_key(
    step: Step,
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    on_change: &ValueChange,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    match step {
        Step::Unit(sign) => runtime.nudge(sign),
        Step::Page(sign) => runtime.nudge(sign * PAGE_STEPS),
        Step::Start => runtime.value.set(runtime.range.min),
        Step::End => runtime.value.set(runtime.range.max),
    }
    on_change(runtime.value.peek());
}
