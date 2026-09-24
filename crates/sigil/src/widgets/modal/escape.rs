use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::reactive::{Signal, on_cleanup};

#[derive(Clone, Copy)]
struct EscapeModal {
    id: u64,
    show: Signal<bool>,
}

thread_local! {
    static ESCAPE_MODALS: RefCell<Vec<EscapeModal>> = const {
        RefCell::new(Vec::new())
    };
}

static NEXT_ESCAPE_MODAL_ID: AtomicU64 = AtomicU64::new(1);

pub(super) fn register_escape_modal(show: Signal<bool>) {
    let id = NEXT_ESCAPE_MODAL_ID.fetch_add(1, Ordering::Relaxed);
    ESCAPE_MODALS.with(|stack| stack.borrow_mut().push(EscapeModal { id, show }));
    on_cleanup(move || {
        ESCAPE_MODALS.with(|stack| {
            stack.borrow_mut().retain(|modal| modal.id != id);
        });
    });
}

pub(super) fn close_top_escape_modal() -> bool {
    ESCAPE_MODALS.with(|stack| {
        let top = stack
            .borrow()
            .iter()
            .rev()
            .copied()
            .find(|modal| modal.show.peek());
        top.is_some_and(|modal| {
            modal.show.set(false);
            true
        })
    })
}
