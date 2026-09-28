use super::*;

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
    let entered = enter_modes(true);
    let inline = INLINE.load(Ordering::Relaxed);
    restore();
    drop(entered);
    assert!(inline, "re-entering an inline host must keep inline mode");
}

#[test]
fn another_thread_does_not_own_the_terminal() {
    set_owner(Some(std::thread::current().id()));
    let elsewhere = std::thread::spawn(owns_terminal).join().unwrap_or(true);
    let here = owns_terminal();
    set_owner(None);
    assert!(
        here && !elsewhere,
        "only the entering thread restores on panic"
    );
}
