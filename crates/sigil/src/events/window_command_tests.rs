use crate::app::{ResizeDirection, WindowCommand};

use super::take_window_commands;
use crate::app::window::queue_command;

#[test]
fn queued_commands_drain_in_order() {
    queue_command(None, WindowCommand::Minimize);
    queue_command(None, WindowCommand::ToggleMaximize);
    queue_command(None, WindowCommand::DragResize(ResizeDirection::SouthEast));

    assert_eq!(
        take_window_commands(),
        vec![
            (None, WindowCommand::Minimize),
            (None, WindowCommand::ToggleMaximize),
            (None, WindowCommand::DragResize(ResizeDirection::SouthEast)),
        ]
    );
}

#[test]
fn drain_leaves_the_queue_empty() {
    queue_command(None, WindowCommand::Close);

    let _ = take_window_commands();

    assert!(take_window_commands().is_empty());
}
