use std::rc::Rc;

use crate::events::{A11yEdit, with_handlers};
use crate::events::{Key, KeyEvent};
use crate::kernel::NodeBuilder;

use crate::widgets::interaction::{DragEnd, InteractionState, ValueStep, pointer_drag, step_keys};

use super::builder::{SliderRuntime, ValueChange};

const SLIDER_KEYS: [(Key, ValueStep); 8] = [
    (Key::ArrowRight, ValueStep::Unit(1.0)),
    (Key::ArrowUp, ValueStep::Unit(1.0)),
    (Key::ArrowLeft, ValueStep::Unit(-1.0)),
    (Key::ArrowDown, ValueStep::Unit(-1.0)),
    (Key::PageUp, ValueStep::Page(1.0)),
    (Key::PageDown, ValueStep::Page(-1.0)),
    (Key::Home, ValueStep::Start),
    (Key::End, ValueStep::End),
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
            let start = runtime.value.peek();
            interaction.drag(true);
            runtime.set_from_position(event);
            Some(start)
        },
        move |_, event| {
            if !runtime.disabled() {
                runtime.set_from_position(event);
                event.stop_propagation();
            }
        },
        move |ended, event| {
            interaction.drag(false);
            let _released = interaction.release_pointer(event);
            match ended {
                DragEnd::Release => on_change(runtime.value.peek()),
                DragEnd::Cancel(start) => runtime.value.set(start),
                DragEnd::Idle => {}
            }
        },
    )
}

fn change_key(
    step: ValueStep,
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    on_change: &ValueChange,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    match step {
        ValueStep::Unit(sign) => runtime.nudge(sign),
        ValueStep::Page(sign) => runtime.nudge(sign * PAGE_STEPS),
        ValueStep::Start => runtime.value.set(runtime.range.min),
        ValueStep::End => runtime.value.set(runtime.range.max),
    }
    on_change(runtime.value.peek());
}
