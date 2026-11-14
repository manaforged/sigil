use std::cell::Cell;
use std::rc::Rc;

use crate::events::{A11yEdit, DragPhase, KeyEvent, NamedKey, PointerEvent, ValueStep};
use crate::node::{NodeBuilder, NodeId};
use crate::reactive::{Signal, signal};
use crate::widgets::Widget;

use super::builder::{SliderRuntime, ValueChange};

const SLIDER_KEYS: [(NamedKey, ValueStep); 8] = [
    (NamedKey::ArrowRight, ValueStep::Unit(1.0)),
    (NamedKey::ArrowUp, ValueStep::Unit(1.0)),
    (NamedKey::ArrowLeft, ValueStep::Unit(-1.0)),
    (NamedKey::ArrowDown, ValueStep::Unit(-1.0)),
    (NamedKey::PageUp, ValueStep::Page(1.0)),
    (NamedKey::PageDown, ValueStep::Page(-1.0)),
    (NamedKey::Home, ValueStep::Start),
    (NamedKey::End, ValueStep::End),
];

const PAGE_STEPS: f32 = 10.0;

#[derive(Clone, Copy)]
pub(super) struct ThumbSignals {
    pub(super) hovered: Signal<bool>,
    pub(super) focused: Signal<bool>,
    pub(super) pressed: Signal<bool>,
    pub(super) dragging: Signal<bool>,
}

impl ThumbSignals {
    pub(super) fn new() -> Self {
        Self {
            hovered: signal(false),
            focused: signal(false),
            pressed: signal(false),
            dragging: signal(false),
        }
    }
}

pub(super) fn attach_key_handlers(
    node: NodeBuilder,
    runtime: SliderRuntime,
    on_change: ValueChange,
) -> NodeBuilder {
    let change = Rc::clone(&on_change);
    node.step_keys(SLIDER_KEYS, move |step, event| {
        change_key(step, event, runtime, &change);
    })
    .a11y_edit(move |edit| handle_a11y_edit(edit, runtime, &on_change))
}

fn handle_a11y_edit(edit: &A11yEdit, runtime: SliderRuntime, on_change: &ValueChange) -> bool {
    match edit {
        A11yEdit::Increment | A11yEdit::Decrement => {
            if !runtime.disabled() {
                let sign = if matches!(edit, A11yEdit::Increment) {
                    1.0
                } else {
                    -1.0
                };
                runtime.nudge(sign);
                on_change(runtime.value.peek());
            }
            true
        }
        A11yEdit::Number(raw) => {
            if !runtime.disabled() {
                runtime.value.set(runtime.quantize(*raw));
                on_change(runtime.value.peek());
            }
            true
        }
        _ => false,
    }
}

#[derive(Clone)]
pub(super) struct Grip {
    dragging: Signal<bool>,
    focus: Option<Rc<Cell<Option<NodeId>>>>,
}

impl Grip {
    pub(super) const fn new(dragging: Signal<bool>) -> Self {
        Self {
            dragging,
            focus: None,
        }
    }

    pub(super) const fn focused(dragging: Signal<bool>, focus: Rc<Cell<Option<NodeId>>>) -> Self {
        Self {
            dragging,
            focus: Some(focus),
        }
    }

    fn focus(&self) {
        if let Some(id) = self.focus.as_ref().and_then(|slot| slot.get()) {
            id.focus();
        }
    }
}

#[derive(Clone, Copy)]
struct DragSession {
    thumb: SliderRuntime,
    start: f32,
    overlapped: bool,
}

pub(super) fn attach_pointer_drag(
    node: NodeBuilder,
    runtime: SliderRuntime,
    grips: [Grip; 2],
    on_change: ValueChange,
) -> NodeBuilder {
    let session = Rc::new(Cell::new(None::<DragSession>));
    node.pointer_drag(move |phase, event| match phase {
        DragPhase::Start => {
            if runtime.disabled() {
                event.prevent_default();
                return;
            }
            let thumb = runtime.on(runtime.value.nearest(runtime.value_at(event)));
            if let Some(grip) = grips.get(thumb.value.index()) {
                grip.focus();
                grip.dragging.set(true);
            }
            let start = thumb.value.peek();
            thumb.value.set(thumb.value_at(event));
            session.set(Some(DragSession {
                thumb,
                start,
                overlapped: thumb.value.overlapped(),
            }));
        }
        DragPhase::Move => {
            let Some(mut active) = session.get() else {
                return;
            };
            if !active.thumb.disabled() {
                active.thumb = drag_to(active.thumb, &grips, event, active.overlapped);
                session.set(Some(active));
                event.stop_propagation();
            }
        }
        DragPhase::End | DragPhase::Cancel => {
            let Some(active) = session.take() else {
                return;
            };
            if let Some(grip) = grips.get(active.thumb.value.index()) {
                grip.dragging.set(false);
            }
            if phase == DragPhase::Cancel {
                active.thumb.value.set(active.start);
            } else {
                on_change(active.thumb.value.peek());
            }
        }
    })
}

fn drag_to(
    held: SliderRuntime,
    grips: &[Grip; 2],
    event: &PointerEvent,
    overlapped: bool,
) -> SliderRuntime {
    let at = held.value_at(event);
    let thumb = if overlapped {
        held.on(held.value.toward(at))
    } else {
        held
    };
    if thumb.value.index() != held.value.index() {
        if let Some(grip) = grips.get(held.value.index()) {
            grip.dragging.set(false);
        }
        if let Some(grip) = grips.get(thumb.value.index()) {
            grip.dragging.set(true);
            grip.focus();
        }
    }
    thumb.value.set(at);
    thumb
}

fn change_key(
    step: ValueStep,
    event: &mut KeyEvent,
    runtime: SliderRuntime,
    on_change: &ValueChange,
) {
    if runtime.disabled() {
        return;
    }
    event.stop_propagation();
    match step {
        ValueStep::Unit(sign) => runtime.nudge(sign),
        ValueStep::Page(sign) => runtime.nudge(sign * PAGE_STEPS),
        ValueStep::Start => runtime.value.set(runtime.range.min),
        ValueStep::End => runtime.value.set(runtime.range.max),
    }
    on_change(runtime.value.peek());
}
