use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;

thread_local! {
    static PATHS: RefCell<VecDeque<Option<PathBuf>>> = const { RefCell::new(VecDeque::new()) };
    static PATH_LISTS: RefCell<VecDeque<Option<Vec<PathBuf>>>> =
        const { RefCell::new(VecDeque::new()) };
    static ALERTS: RefCell<VecDeque<bool>> = const { RefCell::new(VecDeque::new()) };
}

pub fn push_path(response: Option<PathBuf>) {
    PATHS.with(|queue| queue.borrow_mut().push_back(response));
}

pub fn push_paths(response: Option<Vec<PathBuf>>) {
    PATH_LISTS.with(|queue| queue.borrow_mut().push_back(response));
}

pub fn push_alert(ok: bool) {
    ALERTS.with(|queue| queue.borrow_mut().push_back(ok));
}

pub fn reset() {
    PATHS.with(|queue| queue.borrow_mut().clear());
    PATH_LISTS.with(|queue| queue.borrow_mut().clear());
    ALERTS.with(|queue| queue.borrow_mut().clear());
}

pub(super) fn take_alert() -> Option<bool> {
    ALERTS.with(|queue| queue.borrow_mut().pop_front())
}

#[allow(
    clippy::option_option,
    reason = "the option-of-option mirrors the three-state input the caller passes"
)]
pub(super) fn take_path() -> Option<Option<PathBuf>> {
    PATHS.with(|queue| queue.borrow_mut().pop_front())
}

#[allow(
    clippy::option_option,
    reason = "the option-of-option mirrors the three-state input the caller passes"
)]
pub(super) fn take_paths() -> Option<Option<Vec<PathBuf>>> {
    PATH_LISTS.with(|queue| queue.borrow_mut().pop_front())
}
