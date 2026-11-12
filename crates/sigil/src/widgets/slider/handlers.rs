use std::cell::Cell;
use std::rc::Rc;

use crate::events::request_focus;
use crate::events::{A11yEdit, with_handlers};
use crate::events::{KeyEvent, NamedKey, PointerEvent};
use crate::kernel::{NodeBuilder, NodeId};

use crate::widgets::interaction::{DragEnd, InteractionState, ValueStep, pointer_drag, step_keys};

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

#[derive(Clone)]
pub(super) struct Grip {
    interaction: InteractionState,
    focus: Option<Rc<Cell<Option<NodeId>>>>,
}

impl Grip {
    pub(super) const fn new(interaction: InteractionState) -> Self {
        Self {
            interaction,
            focus: None,
        }
    }

    pub(super) const fn focused(
        interaction: InteractionState,
        focus: Rc<Cell<Option<NodeId>>>,
    ) -> Self {
        Self {
            interaction,
            focus: Some(focus),
        }
    }
}

pub(super) fn attach_pointer_drag(
    node: NodeBuilder,
    runtime: SliderRuntime,
    grips: [Grip; 2],
    on_change: ValueChange,
) -> NodeBuilder {
    let grips = Rc::new(grips);
    let (begin, moved, finish) = (Rc::clone(&grips), Rc::clone(&grips), grips);
    let active = Rc::new(Cell::new(runtime));
    let (opened, moving, closed) = (Rc::clone(&active), Rc::clone(&active), active);
    pointer_drag(
        node,
        move |event| {
            if runtime.disabled() {
                return None;
            }
            let thumb = runtime.on(runtime.value.nearest(runtime.value_at(event)));
            let grip = begin.get(thumb.value.index())?;
            if let Some(id) = grip.focus.as_ref().and_then(|slot| slot.get()) {
                request_focus(id);
            }
            opened.set(thumb);
            let start = thumb.value.peek();
            grip.interaction.drag(true);
            thumb.value.set(thumb.value_at(event));
            Some((start, thumb.value.overlapped()))
        },
        move |&(_, overlapped), event| {
            if !moving.get().disabled() {
                moving.set(drag_to(moving.get(), &moved, event, overlapped));
                event.stop_propagation();
            }
        },
        move |ended, event| {
            let thumb = closed.get();
            if let Some(grip) = finish.get(thumb.value.index()) {
                grip.interaction.drag(false);
                let _released = grip.interaction.release_pointer(event);
            }
            match ended {
                DragEnd::Release => on_change(thumb.value.peek()),
                DragEnd::Cancel((start, _)) => thumb.value.set(start),
                DragEnd::Idle => {}
            }
        },
    )
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
            grip.interaction.drag(false);
        }
        if let Some(grip) = grips.get(thumb.value.index()) {
            grip.interaction.drag(true);
            if let Some(id) = grip.focus.as_ref().and_then(|slot| slot.get()) {
                request_focus(id);
            }
        }
    }
    thumb.value.set(at);
    thumb
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
