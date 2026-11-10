use std::cell::{Cell, RefCell};

use crate::kernel::NodeId;
use crate::reactive::Signal;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Autofocus;

thread_local! {
    static PENDING: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

/// Asks the host to focus a node on the next frame.
pub fn request_focus(id: NodeId) {
    PENDING.with(|cell| cell.borrow_mut().push(id));
}

pub(crate) fn take_focus_requests() -> Vec<NodeId> {
    PENDING.take()
}

pub(crate) fn swap_focus_requests(requests: Vec<NodeId>) -> Vec<NodeId> {
    PENDING.replace(requests)
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
