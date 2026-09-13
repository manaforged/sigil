use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use sigil::reactive::on_cleanup;

thread_local! {
    static TIMERS: RefCell<Vec<Timer>> = const { RefCell::new(Vec::new()) };
}

struct Timer {
    period: f32,
    left: f32,
    run: Box<dyn FnMut()>,
    alive: Rc<Cell<bool>>,
}

pub fn every<F>(period: Duration, f: F)
where
    F: FnMut() + 'static,
{
    let period = period.as_secs_f32().max(f32::EPSILON);
    let alive = Rc::new(Cell::new(true));
    let cleanup = Rc::clone(&alive);
    on_cleanup(move || cleanup.set(false));
    TIMERS.with_borrow_mut(|timers| {
        timers.push(Timer {
            period,
            left: period,
            run: Box::new(f),
            alive,
        });
    });
}

pub fn clear() {
    TIMERS.with_borrow_mut(Vec::clear);
}

pub(crate) fn advance(dt: f32) {
    let mut timers = TIMERS.with_borrow_mut(std::mem::take);
    timers.retain_mut(|timer| {
        if !timer.alive.get() {
            return false;
        }
        timer.left -= dt;
        if timer.left <= 0.0 {
            timer.left = timer.period;
            (timer.run)();
        }
        true
    });
    TIMERS.with_borrow_mut(|next| {
        timers.append(next);
        *next = timers;
    });
}
