use crate::app::{Window, WindowCommand};

#[must_use]
pub(crate) fn take_window_commands() -> Vec<(Option<Window>, WindowCommand)> {
    crate::app::window::take_commands()
}

#[cfg(test)]
#[path = "window_command_tests.rs"]
mod tests;
