use super::*;

#[test]
fn reset_clears_queued_commands() {
    reset();
    let window = allocate_window_id();
    window.command(WindowCommand::Minimize);
    queue_command(None, WindowCommand::Show);
    reset();
    assert!(take_commands().is_empty());
}
