//! Periodic callbacks driven by the frame loop.
//!
//! The terminal host already ticks at a fixed rate to drive animations. A
//! timer is a countdown on that same tick, so a headless [`step`](crate::step)
//! advances one exactly as a live frame does and a test can drive a clock
//! without waiting on a real one.

use std::cell::RefCell;
use std::time::Duration;

thread_local! {
    static TIMERS: RefCell<Vec<Timer>> = const { RefCell::new(Vec::new()) };
}

struct Timer {
    period: f32,
    left: f32,
    run: Box<dyn FnMut()>,
}

/// Run `f` every `period` for as long as the host runs.
///
/// Register from the tree builder. A timer lives until the host stops, so
/// register on mount rather than per frame.
pub fn every<F>(period: Duration, f: F)
where
    F: FnMut() + 'static,
{
    let period = period.as_secs_f32().max(f32::EPSILON);
    TIMERS.with_borrow_mut(|timers| {
        timers.push(Timer {
            period,
            left: period,
            run: Box::new(f),
        });
    });
}

/// Drop every registered timer.
pub fn clear() {
    TIMERS.with_borrow_mut(Vec::clear);
}

/// Advance every timer by `dt` seconds and run the ones that come due.
pub(crate) fn advance(dt: f32) {
    // Take the list: a callback may register another timer, and holding the
    // borrow across the call would panic.
    let mut timers = TIMERS.with_borrow_mut(std::mem::take);
    for timer in &mut timers {
        timer.left -= dt;
        if timer.left <= 0.0 {
            timer.left = timer.period;
            (timer.run)();
        }
    }
    TIMERS.with_borrow_mut(|next| {
        timers.append(next);
        *next = timers;
    });
}
