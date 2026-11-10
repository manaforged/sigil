use std::cell::RefCell;

use crate::reactive::Runtime;
use crate::style::Theme;

thread_local! {
    static PENDING: RefCell<Option<Theme>> = const { RefCell::new(None) };
}

impl Theme {
    /// Applies this theme to the running app at the next frame.
    pub fn apply(self) {
        PENDING.with(|pending| *pending.borrow_mut() = Some(self));
        Runtime::mark_frame_dirty();
    }
}

pub(super) fn take() -> Option<Theme> {
    PENDING.take()
}

pub(super) fn swap_pending(theme: Option<Theme>) -> Option<Theme> {
    PENDING.replace(theme)
}
