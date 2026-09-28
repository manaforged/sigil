use super::*;

static SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn committing_lines_above_the_live_region_repaints_it() -> io::Result<()> {
    LIVE.with(|live| live.set(true));
    INLINE.store(true, Ordering::Relaxed);
    REDRAW.with(|flag| flag.set(false));
    let printed = print_above("committed");
    let pending = REDRAW.with(Cell::get);
    LIVE.with(|live| live.set(false));
    INLINE.store(false, Ordering::Relaxed);
    printed?;
    assert!(
        pending,
        "the rows above the live region scrolled, so the next paint must repaint it"
    );
    Ok(())
}

#[test]
fn reentry_restores_inline_mode() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let entered = enter_modes(true);
    let inline = INLINE.load(Ordering::Relaxed);
    restore();
    drop(entered);
    assert!(inline, "re-entering an inline host must keep inline mode");
}

#[test]
fn another_thread_does_not_own_the_terminal() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    set_owner(Some(std::thread::current().id()));
    let elsewhere = std::thread::spawn(owns_terminal).join().unwrap_or(true);
    let here = owns_terminal();
    set_owner(None);
    assert!(
        here && !elsewhere,
        "only the entering thread restores on panic"
    );
}

#[test]
fn each_tracking_mode_resets_then_enables_only_its_reports() -> io::Result<()> {
    let expected = [
        (MouseTracking::Off, ""),
        (MouseTracking::Clicks, "\x1b[?1000h\x1b[?1006h"),
        (MouseTracking::Drags, "\x1b[?1000h\x1b[?1002h\x1b[?1006h"),
        (
            MouseTracking::Motion,
            "\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h",
        ),
    ];
    for (tracking, enable) in expected {
        let mut out = Vec::new();
        write_mouse(&mut out, tracking)?;
        assert_eq!(
            String::from_utf8_lossy(&out),
            format!("\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l{enable}"),
            "{tracking:?}"
        );
    }
    Ok(())
}

#[test]
fn a_panic_on_another_thread_leaves_the_terminal_alone() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    set_owner(Some(std::thread::current().id()));
    RESTORED.store(false, Ordering::Relaxed);
    let hook = HookGuard::install();
    let panicked = std::thread::spawn(|| std::panic::resume_unwind(Box::new("worker failed")))
        .join()
        .is_err();
    let restored = RESTORED.load(Ordering::Relaxed);
    drop(hook);
    RESTORED.store(true, Ordering::Relaxed);
    set_owner(None);
    assert!(panicked, "the worker thread must panic");
    assert!(
        !restored,
        "a worker panic must not restore the terminal the UI thread still drives"
    );
}
