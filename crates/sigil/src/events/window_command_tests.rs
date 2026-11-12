use crate::app::{ResizeDirection, WindowCommand};

use super::{take_window_commands, window_command};

#[test]
fn queued_commands_drain_in_order() {
    window_command(WindowCommand::Minimize);
    window_command(WindowCommand::ToggleMaximize);
    window_command(WindowCommand::DragResize(ResizeDirection::SouthEast));

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
    window_command(WindowCommand::Close);

    let _ = take_window_commands();

    assert!(take_window_commands().is_empty());
}
