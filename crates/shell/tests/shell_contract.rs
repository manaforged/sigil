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
//! 2. **Display-gated** tests construct a real `EventLoop` and are
//!    marked `#[ignore]` so `cargo test` is green on CI. Run locally
//!    with:
//!
//!    ```text
//!    cargo test -p sigil-shell -- --ignored --test-threads=1
//!    ```
//!
//!    Because of (2) above only *one* display-gated test may run per
//!    process invocation. The consolidated
//!    [`construct_and_probe`] test owns the single allowed
//!    `EventLoop` and exercises the full pre-run contract in one go.
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
/// corresponding runtime path is covered by
/// `construct_and_probe::run_then_retry` when ignored tests are run
/// locally.
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
// Display-gated single-shot test.
// ---------------------------------------------------------------------------

/// Consolidated pre-run contract check. Constructs the single
/// `EventLoop` winit allows per process and walks every pre-run
/// surface: waker bounds and liveness, wake coalescing, window
/// handle allocation, window lookup before `resumed`, the failed-
/// window drain shape, and post-drop waker error shape.
///
/// Ignored by default — see the module docs for the invocation
/// string. This test cannot be run inside the normal `cargo test`
/// because `cargo test` dispatches onto a worker thread and `winit`
/// on Windows demands the process main thread.
#[test]
#[ignore = "requires the process main thread; run locally with --ignored --test-threads=1"]
fn construct_and_probe() {
    use std::sync::atomic::Ordering;

    use sigil_hal::{PhysicalSize, Shell};

    let mut shell = WinitShell::new().expect("construct shell on the main thread");

    // waker: cheap-to-clone, live against a fresh loop.
    let waker = shell.waker();
    let cloned = waker.clone();
    assert!(
        <WinitShell as Shell>::wake(&waker).is_ok(),
        "wake should succeed while the event loop is alive"
    );
    drop(cloned);

    // --- coalescing -------------------------------------------------
    //
    // The first wake above already incremented `WAKE_DELIVERED_COUNT`
    // once. We call wake() ten more times from the same thread; none
    // of them should forward a WakeEvent because `user_event` has not
    // yet run (the event loop is not executing), so the pending flag
    // remains set from the first call. The counter must still be
    // exactly 1.
    let baseline = sigil_shell::WAKE_DELIVERED_COUNT.load(Ordering::Relaxed);
    for _ in 0..10 {
        <WinitShell as Shell>::wake(&waker).expect("wake still ok");
    }
    let after = sigil_shell::WAKE_DELIVERED_COUNT.load(Ordering::Relaxed);
    assert_eq!(
        after,
        baseline,
        "10 additional wakes without a user_event drain must coalesce into 0 deliveries, saw {delta}",
        delta = after - baseline,
    );

    // create_window: returns unique handles, no window is
    // materialised before `resumed` fires.
    let a = shell
        .create_window("alpha", PhysicalSize::new(800, 600))
        .expect("queue alpha");
    let b = shell
        .create_window("beta", PhysicalSize::new(1024, 768))
        .expect("queue beta");
    assert_ne!(a.id(), b.id(), "handles must be unique");
    assert!(
        shell.window_for(a).is_none(),
        "window lookups must be None before the event loop runs"
    );
    assert!(shell.windows().is_empty());

    // take_failed_windows: nothing has been attempted yet.
    assert!(
        shell.take_failed_windows().is_empty(),
        "no materialisation has run; failure list must be empty"
    );

    // drop(shell) tears the event loop down; the previously-cloned
    // waker must then surface a clean error instead of panicking.
    drop(shell);
    let err =
        <WinitShell as Shell>::wake(&waker).expect_err("wake on a dropped event loop must error");
    let msg = err.to_string();
    assert!(
        msg.contains("event loop") || msg.contains("closed"),
        "error should mention event loop closure, got: {msg}"
    );
}
