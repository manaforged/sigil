use std::cell::RefCell;

use crate::reactive::Runtime;
use crate::style::Theme;

thread_local! {
    static PENDING: RefCell<Option<Theme>> = const { RefCell::new(None) };
}

pub fn set_theme(theme: Theme) {
    PENDING.with(|pending| *pending.borrow_mut() = Some(theme));
    Runtime::mark_frame_dirty();
}

pub(super) fn take() -> Option<Theme> {
    PENDING.with(|pending| pending.borrow_mut().take())
}
