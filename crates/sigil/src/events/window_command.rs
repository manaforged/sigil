use crate::app::{WindowCommand, WindowId};

pub fn request_window_command(command: WindowCommand) {
    crate::app::window::queue_command(crate::app::current_window(), command);
}

#[must_use]
pub(crate) fn take_window_commands() -> Vec<(Option<WindowId>, WindowCommand)> {
    crate::app::window::take_commands()
}

#[cfg(test)]
#[path = "window_command_tests.rs"]
mod tests;
