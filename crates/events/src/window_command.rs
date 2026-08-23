//! Deferred window-command queue.
//!
//! Widget pointer handlers run without a `&mut Cx` or any window
//! handle, so a custom titlebar button or drag region cannot reach the
//! shell directly. It pushes a [`WindowCommand`] through
//! [`request_window_command`] — a thread-local queue the host frame
//! loop drains once per frame (`sigil-app`'s `drain_events`) and
//! applies through [`sigil_hal::Shell::window_command`]. Same
//! constraint and shape as [`crate::request_focus`]: effects and
//! handlers enqueue, the event drain applies.

use std::cell::RefCell;

use sigil_hal::WindowCommand;

thread_local! {
    static PENDING: RefCell<Vec<WindowCommand>> = const { RefCell::new(Vec::new()) };
}

/// Queue a window command, applied to the app's window when the host
/// drains the queue at the end of this frame's event dispatch.
pub fn request_window_command(command: WindowCommand) {
    PENDING.with(|cell| cell.borrow_mut().push(command));
}

/// Drain every queued command, leaving the queue empty. Called by the
/// host frame loop; commands naming a window that has not materialised
/// yet are dropped at apply time.
#[must_use]
pub fn take_window_commands() -> Vec<WindowCommand> {
    PENDING.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}

#[cfg(test)]
#[path = "window_command_tests.rs"]
mod tests;
