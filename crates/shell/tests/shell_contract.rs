#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Contract tests for the winit [`Shell`] adapter.
//!
//! A real winit event loop is hostile to `cargo test`:
//!
//! 1. On Windows, `EventLoop::new()` panics unless invoked from the
//!    process's *main* thread. `cargo test` always dispatches tests
//!    on worker threads, even under `--test-threads=1`.
//! 2. `winit` refuses to construct more than one `EventLoop` per
//!    process — the second attempt returns `EventLoopError::
//!    RecreationAttempt`. That forbids multiple tests in the same
//!    binary that each own an event loop.
//! 3. `run_app` blocks until the OS requests exit; driving it in a
//!    test harness is not reproducible without a synthetic event
//!    source.
//!
//! We split the suite into two tiers, mirroring the pattern used by
//! `sigil-renderer`'s GPU-gated tests:
//!
//! 1. **Unconditional** tests exercise compile-time bounds and error
//!    surface shape. These always run.
//! 2. **Display-gated** construction lives in the sibling `display_gate`
//!    target (`harness = false`), which runs on the process main thread
//!    — the only thread `AppKit` accepts. It is skipped unless
//!    `SIGIL_DISPLAY_GATE=1` is set:
//!
//!    ```text
//!    SIGIL_DISPLAY_GATE=1 cargo test -p sigil-shell --test display_gate
//!    ```
//!
//! See DR-002 for the trait contract being verified.

// `winit` is a regular dep of `sigil-shell`; integration tests inherit
// its dep graph even though they don't reference winit directly.

use sigil_shell::{ShellError, WinitShell, WinitWaker};

/// The waker type must be `Send + Sync + Clone + 'static` so it can
/// cross thread boundaries into `sigil-async`'s background Tokio tasks.
#[test]
fn waker_is_send_sync_clone_static() {
    fn assert_bounds<T: Send + Sync + Clone + 'static>() {}
    assert_bounds::<<WinitShell as sigil_hal::Shell>::Waker>();
    assert_bounds::<WinitWaker>();
}

/// The adapter error must implement `std::error::Error` and the
/// `Send + Sync + 'static` bound required by the `Shell::Error`
/// associated type.
#[test]
fn shell_error_implements_std_error() {
    fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
    assert_error::<ShellError>();
}

/// `ShellError::AlreadyRun` must carry a user-facing message. The
/// corresponding runtime path is covered by the display-gated
/// `display_gate` target when run locally on a GUI machine.
#[test]
fn already_run_error_has_display_message() {
    let err = ShellError::AlreadyRun;
    assert_eq!(err.to_string(), "shell run called twice");
}

/// `EventLoopClosed` similarly has a stable `Display` rendering so
/// background tasks can log it without pulling in the winit error
/// vocabulary.
#[test]
fn event_loop_closed_error_has_display_message() {
    let err = ShellError::EventLoopClosed;
    assert_eq!(err.to_string(), "event loop already exited");
}

/// `take_failed_windows` drains an owned `Vec`. Verified structurally
/// here — the live event-loop path is exercised inside the
/// display-gated `construct_and_probe` test.
///
/// This is a compile-time check: if the signature regresses to a
/// slice or an iterator, the test stops compiling.
#[test]
fn take_failed_windows_signature_is_owned_vec() {
    #[allow(dead_code)]
    fn probe(shell: &mut WinitShell) -> Vec<(sigil_shell::WinitWindowHandle, String)> {
        shell.take_failed_windows()
    }
}

// ---------------------------------------------------------------------------
// Display-gated construction lives in `tests/display_gate.rs`
// (`harness = false`): winit demands the process main thread on macOS,
// and libtest dispatches tests onto worker threads in every mode.
// ---------------------------------------------------------------------------
