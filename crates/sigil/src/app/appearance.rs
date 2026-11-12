use std::cell::RefCell;

use crate::reactive::Runtime;
use crate::style::Theme;

thread_local! {
    static PENDING: RefCell<Option<Box<Theme>>> = const { RefCell::new(None) };
}

impl Theme {
    /// Applies this theme to the running app at the next frame.
    pub fn apply(self) {
        PENDING.with(|pending| *pending.borrow_mut() = Some(Box::new(self)));
        Runtime::mark_frame_dirty();
    }
}

pub(super) fn take() -> Option<Theme> {
    PENDING.take().map(|theme| *theme)
}

pub(super) fn swap_pending(theme: Option<Box<Theme>>) -> Option<Box<Theme>> {
    PENDING.replace(theme)
}
