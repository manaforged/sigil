use std::cell::RefCell;

use crate::kernel::NodeId;

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

pub(crate) fn take_reveal_requests() -> Vec<NodeId> {
    REVEAL.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}
