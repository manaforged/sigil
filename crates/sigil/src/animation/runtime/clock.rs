use core::cell::RefCell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::reactive::frame::{FrameSource, SourceId, try_with_sources, with_sources};
use crate::reactive::{Owner, Runtime, Signal, batch, cleanup, signal};

pub(crate) struct ClockSource {
    seconds: Signal<f64>,
    elapsed: f64,
}

fn register_source(source: FrameSource) {
    let id = with_sources(|sources| sources.map.insert(source));
    cleanup(move || {
        let _ = try_with_sources(|sources| sources.map.remove(id));
    });
    Runtime::mark_frame_dirty();
}

/// Returns a signal that holds the animation clock.
#[must_use]
#[track_caller]
pub fn clock() -> Signal<f64> {
    assert!(
        Owner::current().is_some(),
        "clock called outside a reactive scope"
    );
    let seconds = signal(0.0_f64);
    register_source(FrameSource::Clock(ClockSource {
        seconds,
        elapsed: 0.0,
    }));
    seconds
}

/// Registers `tick` to run on every frame with the elapsed seconds since the previous frame.
#[track_caller]
pub fn every_frame(tick: impl FnMut(f32) + 'static) {
    assert!(
        Owner::current().is_some(),
        "every_frame called outside a reactive scope"
    );
    let callback: Rc<RefCell<dyn FnMut(f32)>> = Rc::new(RefCell::new(tick));
    register_source(FrameSource::Callback(callback));
}

kittest_pub! {
    /// Returns the number of frame clocks, frame callbacks, and perpetual animations that are running.
    #[must_use]
    pub fn running_clocks() -> usize {
        with_sources(|sources| {
            sources
                .map
                .values()
                .filter(|source| matches!(source, FrameSource::Clock(_) | FrameSource::Callback(_)))
                .count()
        }) + crate::animation::runtime::state::perpetual_count()
    }
}

pub(crate) fn tick_clocks(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 || running_clocks() == 0 {
        return;
    }
    batch(|| {
        let callbacks = with_sources(|sources| {
            let mut callbacks = SmallVec::<[(SourceId, Rc<RefCell<dyn FnMut(f32)>>); 4]>::new();
            sources.map.retain(|id, source| match source {
                FrameSource::Clock(clock) => {
                    clock.elapsed += f64::from(dt);
                    clock.seconds.try_set(clock.elapsed)
                }
                FrameSource::Callback(callback) => {
                    callbacks.push((id, Rc::clone(callback)));
                    true
                }
                _ => true,
            });
            callbacks
        });
        for (id, callback) in callbacks {
            if with_sources(|sources| sources.map.contains_key(id)) {
                (callback.borrow_mut())(dt);
            }
        }
        Runtime::mark_frame_dirty();
    });
}
