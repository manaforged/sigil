use std::cell::{Cell, RefCell};

use crate::kernel::NodeId;
use crate::reactive::Signal;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Autofocus;

thread_local! {
    static PENDING: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
    static REVEAL: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

pub fn request_focus(id: NodeId) {
    PENDING.with(|cell| cell.borrow_mut().push(id));
}

pub fn request_reveal(id: NodeId) {
    REVEAL.with(|cell| cell.borrow_mut().push(id));
}

pub(crate) fn take_focus_requests() -> Vec<NodeId> {
    PENDING.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}

pub(crate) fn focus_on_trigger(trigger: Signal<u64>, focus: impl Fn() + 'static) {
    let last = Cell::new(trigger.peek());
    crate::reactive::effect_named("sigil::focus_when", move || {
        let generation = trigger.get();
        if generation != last.replace(generation) {
            focus();
        }
    });
}

pub(crate) fn take_reveal_requests() -> Vec<NodeId> {
    REVEAL.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}
