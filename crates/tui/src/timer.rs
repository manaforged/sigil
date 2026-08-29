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

pub fn clear() {
    TIMERS.with_borrow_mut(Vec::clear);
}

pub(crate) fn advance(dt: f32) {
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
