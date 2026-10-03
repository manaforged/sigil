use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

static FALLBACK: AtomicBool = AtomicBool::new(false);

thread_local! {
    static TARGET: RefCell<Option<Rc<Cell<bool>>>> = const { RefCell::new(None) };
}

pub fn request_exit() {
    let hosted = TARGET.with(|slot| {
        slot.borrow().as_ref().is_some_and(|cell| {
            cell.set(true);
            true
        })
    });
    if !hosted {
        FALLBACK.store(true, Ordering::SeqCst);
    }
}

#[must_use]
pub(crate) fn take_exit() -> bool {
    if let Some(cell) = TARGET.with(|slot| slot.borrow().clone()) {
        return cell.replace(false);
    }
    FALLBACK.swap(false, Ordering::SeqCst)
}

#[cfg(any(test, feature = "desktop", feature = "terminal"))]
pub(crate) fn clear_exit() {
    if let Some(cell) = TARGET.with(|slot| slot.borrow().clone()) {
        cell.set(false);
    }
    FALLBACK.store(false, Ordering::SeqCst);
}

pub(crate) fn swap_target(target: Option<Rc<Cell<bool>>>) -> Option<Rc<Cell<bool>>> {
    TARGET.with(|slot| core::mem::replace(&mut *slot.borrow_mut(), target))
}

#[cfg(test)]
#[path = "exit_tests.rs"]
mod tests;
