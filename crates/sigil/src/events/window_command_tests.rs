use crate::hal::{ResizeDirection, WindowCommand};

use super::{request_window_command, take_window_commands};

#[test]
fn queued_commands_drain_in_order() {
    request_window_command(WindowCommand::Minimize);
    request_window_command(WindowCommand::ToggleMaximize);
    request_window_command(WindowCommand::DragResize(ResizeDirection::SouthEast));

    assert_eq!(
        take_window_commands(),
        vec![
            WindowCommand::Minimize,
            WindowCommand::ToggleMaximize,
            WindowCommand::DragResize(ResizeDirection::SouthEast),
        ]
    );
}

#[test]
fn drain_leaves_the_queue_empty() {
    request_window_command(WindowCommand::Close);

    let _ = take_window_commands();

    assert!(take_window_commands().is_empty());
}
