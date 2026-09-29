#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::geometry::PhysicalSize;
use sigil::hal::Shell;
use sigil::shell::WinitShell;
use std::sync::atomic::Ordering;

#[allow(
    clippy::print_stdout,
    reason = "harness = false target: main() is its own reporter"
)]
fn main() {
    if std::env::var_os("SIGIL_DISPLAY_GATE").is_none() {
        println!("display_gate: skipped (set SIGIL_DISPLAY_GATE=1 on a GUI machine)");
        return;
    }

    let mut shell = WinitShell::new().expect("construct shell on the main thread");
    wake_contract(&shell);
    window_contract(&mut shell);
    let waker = shell.waker();
    drop(shell);
    drop_contract(&waker);

    println!("display_gate: PASS");
}

fn wake_contract(shell: &WinitShell) {
    let waker = shell.waker();
    let cloned = waker.clone();
    assert!(
        <WinitShell as Shell>::wake(&waker).is_ok(),
        "wake should succeed while the event loop is alive"
    );
    drop(cloned);

    let baseline = sigil::shell::WAKE_DELIVERED_COUNT.load(Ordering::Relaxed);
    for _ in 0..10 {
        <WinitShell as Shell>::wake(&waker).expect("wake still ok");
    }
    let after = sigil::shell::WAKE_DELIVERED_COUNT.load(Ordering::Relaxed);
    assert_eq!(
        after,
        baseline,
        "10 additional wakes without a user_event drain must coalesce into 0 deliveries, saw {delta}",
        delta = after - baseline,
    );
}

fn window_contract(shell: &mut WinitShell) {
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
    assert!(
        shell.take_failed_windows().is_empty(),
        "no materialisation has run; failure list must be empty"
    );
}

fn drop_contract(waker: &<WinitShell as Shell>::Waker) {
    let err = <WinitShell as Shell>::wake(waker).expect_err("wake after drop must error");
    let msg = err.to_string();
    assert!(
        msg.contains("event loop") || msg.contains("closed"),
        "error should mention event loop closure, got: {msg}"
    );
}
