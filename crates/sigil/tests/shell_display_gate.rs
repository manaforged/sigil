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

    if cfg!(any(target_os = "macos", windows)) {
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
const QUIT_TITLE: &str = "sigil quit gate";
const EXIT_HOOK_RAN: &str = "display_gate: exit hook ran";
const RUN_RETURNED_OK: &str = "display_gate: run returned Ok";
#[cfg(windows)]
const BLOCK_REASON: &str = "display_gate: block reason Saving before closing";

const QUIT_DEADLINE: std::time::Duration = std::time::Duration::from_secs(90);

fn quit_contract() {
    let mut command =
        std::process::Command::new(std::env::current_exe().expect("gate binary path"));
    command.env(QUIT_CHILD, "1");
    if cfg!(windows) {
        command.env("SIGIL_BACKEND", "cpu");
    }
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("start the quit child");
    let stdout = drain(child.stdout.take().expect("child stdout"));
    let stderr = drain(child.stderr.take().expect("child stderr"));
    let started = std::time::Instant::now();
    let exited = loop {
        if child.try_wait().expect("poll the quit child").is_some() {
            break true;
        }
        if started.elapsed() > QUIT_DEADLINE {
            drop(child.kill());
            drop(child.wait());
            break false;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let stdout = stdout.join().expect("child stdout reader");
    let stderr = stderr.join().expect("child stderr reader");
    assert!(
        exited,
        "the quit child did not exit within {QUIT_DEADLINE:?}; stdout: {stdout} stderr: {stderr}"
    );
    assert!(
        stdout.matches(EXIT_HOOK_RAN).count() == 1,
        "a quit sent to the app must run its exit hook once, child stdout: {stdout} stderr: {stderr}"
    );
    #[cfg(windows)]
    assert!(
        stdout.contains(BLOCK_REASON) && stdout.contains(RUN_RETURNED_OK),
        "a Windows session end must name the block reason and end the event loop, child stdout: {stdout} stderr: {stderr}"
    );
}

fn drain(mut pipe: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = String::new();
        drop(pipe.read_to_string(&mut text));
        text
    })
}

#[allow(
    clippy::print_stdout,
    reason = "harness = false target: the child reports to its parent on stdout"
)]
fn quit_child() {
    let sent = std::cell::Cell::new(false);
    let config = sigil::AppConfig::new(QUIT_TITLE).on_exit(|| {
        println!("{EXIT_HOOK_RAN}");
    });
    let result = sigil::run(config, move || {
        if !sent.replace(true) {
            send_quit();
        }
        sigil::prelude::text("quit gate")
    });
    match result {
        Ok(()) => println!("{RUN_RETURNED_OK}"),
        Err(error) => println!("display_gate: run returned {error}"),
    }
}

#[cfg(target_os = "macos")]
fn send_quit() {
    let script = format!(
        "ObjC.import('AppKit'); $.NSRunningApplication.runningApplicationWithProcessIdentifier({}).terminate",
        std::process::id()
    );
    drop(
        std::process::Command::new("osascript")
            .args(["-l", "JavaScript", "-e", &script])
            .spawn(),
    );
}

#[cfg(windows)]
fn send_quit() {
    drop(std::thread::spawn(end_session));
}

#[cfg(not(any(target_os = "macos", windows)))]
const fn send_quit() {}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "the gate plays the Windows session manager and sends the session-end messages"
)]
#[allow(
    clippy::print_stdout,
    reason = "harness = false target: the child reports to its parent on stdout"
)]
fn end_session() {
    use windows_sys::Win32::System::Shutdown::ShutdownBlockReasonQuery;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        ENDSESSION_CLOSEAPP, FindWindowW, SendMessageW, WM_ENDSESSION, WM_QUERYENDSESSION,
    };
    let title: Vec<u16> = QUIT_TITLE.encode_utf16().chain(Some(0)).collect();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let hwnd = loop {
        // SAFETY: `title` is a NUL-terminated UTF-16 string that outlives the call.
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if !hwnd.is_null() {
            println!("display_gate: quit window found");
            break hwnd;
        }
        if std::time::Instant::now() > deadline {
            println!("display_gate: quit window not found");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let closeapp = ENDSESSION_CLOSEAPP as isize;
    // SAFETY: `hwnd` is this process's live top-level window; SendMessageW waits until its thread handles the message.
    unsafe { SendMessageW(hwnd, WM_QUERYENDSESSION, 0, closeapp) };
    println!("display_gate: query end session answered");
    let mut reason = [0u16; 256];
    let mut len = reason.len() as u32;
    // SAFETY: `reason` and `len` describe a writable UTF-16 buffer.
    if unsafe { ShutdownBlockReasonQuery(hwnd, reason.as_mut_ptr(), &mut len) } != 0 {
        let text: Vec<u16> = reason
            .iter()
            .copied()
            .take_while(|unit| *unit != 0)
            .collect();
        println!(
            "display_gate: block reason {}",
            String::from_utf16_lossy(&text)
        );
    }
    // SAFETY: as above; WPARAM 1 tells the window the session is ending.
    unsafe { SendMessageW(hwnd, WM_ENDSESSION, 1, closeapp) };
    println!("display_gate: end session answered");
}
