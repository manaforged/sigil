use core::cell::RefCell;
use slotmap::{SlotMap, new_key_type};

use crate::reactive::{Owner, Runtime, Signal, batch, on_cleanup, signal};

new_key_type! {
    struct ClockId;
}

struct Clock {
    seconds: Signal<f64>,
    elapsed: f64,
}

thread_local! {
    static CLOCKS: RefCell<SlotMap<ClockId, Clock>> = RefCell::new(SlotMap::with_key());
}

#[must_use]
#[track_caller]
pub fn clock() -> Signal<f64> {
    assert!(
        Owner::current().is_some(),
        "clock called outside a reactive scope"
    );
    let seconds = signal(0.0_f64);
    let id = CLOCKS.with(|cell| {
        cell.borrow_mut().insert(Clock {
            seconds,
            elapsed: 0.0,
        })
    });
    on_cleanup(move || {
        CLOCKS.with(|cell| cell.borrow_mut().remove(id));
    });
    Runtime::mark_frame_dirty();
    seconds
}

#[must_use]
pub fn running_clocks() -> usize {
    CLOCKS.with(|cell| cell.borrow().len())
}

pub(crate) fn tick_clocks(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 || running_clocks() == 0 {
        return;
    }
    batch(|| {
        CLOCKS.with(|cell| {
            cell.borrow_mut().retain(|_, clock| {
                clock.elapsed += f64::from(dt);
                clock.seconds.try_set(clock.elapsed)
            });
        });
        Runtime::mark_frame_dirty();
    });
}
