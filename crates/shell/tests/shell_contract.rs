#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil_shell::{ShellError, WinitShell, WinitWaker};

#[test]
fn waker_is_send_sync_clone_static() {
    fn assert_bounds<T: Send + Sync + Clone + 'static>() {}
    assert_bounds::<<WinitShell as sigil_hal::Shell>::Waker>();
    assert_bounds::<WinitWaker>();
}

#[test]
fn shell_error_implements_std_error() {
    fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
    assert_error::<ShellError>();
}

#[test]
fn already_run_error_has_display_message() {
    let err = ShellError::AlreadyRun;
    assert_eq!(err.to_string(), "shell run called twice");
}

#[test]
fn event_loop_closed_error_has_display_message() {
    let err = ShellError::EventLoopClosed;
    assert_eq!(err.to_string(), "event loop already exited");
}

#[test]
fn take_failed_windows_signature_is_owned_vec() {
    #[allow(dead_code)]
    fn probe(shell: &mut WinitShell) -> Vec<(sigil_shell::WinitWindowHandle, String)> {
        shell.take_failed_windows()
    }
}
