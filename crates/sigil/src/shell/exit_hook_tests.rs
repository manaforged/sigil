use std::cell::Cell;
use std::rc::Rc;

use super::{pending, run, set};

#[test]
fn the_hook_runs_once_when_the_loop_exit_and_a_session_end_both_fire() {
    let calls = Rc::new(Cell::new(0));
    let counted = Rc::clone(&calls);
    set(Box::new(move || counted.set(counted.get() + 1)));

    run();
    run();

    assert_eq!(calls.get(), 1);
    assert!(!pending());
}

#[test]
fn a_running_hook_sees_no_pending_hook_and_can_install_the_next_one() {
    let seen = Rc::new(Cell::new(true));
    let observed = Rc::clone(&seen);
    set(Box::new(move || {
        observed.set(pending());
        set(Box::new(|| {}));
    }));

    run();

    assert!(!seen.get());
    assert!(pending());
    run();
}
