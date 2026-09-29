use std::cell::RefCell;

use crate::app::WindowCommand;

thread_local! {
    static PENDING: RefCell<Vec<WindowCommand>> = const { RefCell::new(Vec::new()) };
}

pub fn request_window_command(command: WindowCommand) {
    PENDING.with(|cell| cell.borrow_mut().push(command));
}

#[must_use]
pub fn take_window_commands() -> Vec<WindowCommand> {
    PENDING.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}

#[cfg(test)]
#[path = "window_command_tests.rs"]
mod tests;
