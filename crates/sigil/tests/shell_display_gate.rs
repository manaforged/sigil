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
    if std::env::var_os(QUIT_CHILD).is_some() {
        return quit_child();
    }
    if std::env::var_os("SIGIL_DISPLAY_GATE").is_none() {
        println!("display_gate: skipped (set SIGIL_DISPLAY_GATE=1 on a GUI machine)");
        return;
    }

    if cfg!(target_os = "macos") {
        quit_contract();
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

const QUIT_CHILD: &str = "SIGIL_DISPLAY_GATE_QUIT_CHILD";
const EXIT_HOOK_RAN: &str = "display_gate: exit hook ran";

fn quit_contract() {
    let output = std::process::Command::new(std::env::current_exe().expect("gate binary path"))
        .env(QUIT_CHILD, "1")
        .output()
        .expect("run the quit child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.matches(EXIT_HOOK_RAN).count() == 1,
        "a quit sent to the app must run its exit hook once, child stdout: {stdout}"
    );
}

#[allow(
    clippy::print_stdout,
    reason = "harness = false target: the child reports to its parent on stdout"
)]
fn quit_child() {
    let sent = std::cell::Cell::new(false);
    let config = sigil::AppConfig::new("quit gate").on_exit(|| {
        println!("{EXIT_HOOK_RAN}");
    });
    let result = sigil::run(config, move || {
        if !sent.replace(true) {
            send_quit(std::process::id());
        }
        sigil::prelude::text("quit gate")
    });
    println!("display_gate: run returned {result:?}");
}

fn send_quit(pid: u32) {
    let script = format!(
        "ObjC.import('AppKit'); $.NSRunningApplication.runningApplicationWithProcessIdentifier({pid}).terminate"
    );
    drop(
        std::process::Command::new("osascript")
            .args(["-l", "JavaScript", "-e", &script])
            .spawn(),
    );
}
