use crate::reactive::frame::{FrameSource, with_sources};
use crate::reactive::{Owner, Runtime, Signal, batch, on_cleanup, signal};

pub(crate) struct ClockSource {
    seconds: Signal<f64>,
    elapsed: f64,
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
    let source = FrameSource::Clock(ClockSource {
        seconds,
        elapsed: 0.0,
    });
    let id = with_sources(|sources| sources.map.insert(source));
    on_cleanup(move || {
        with_sources(|sources| sources.map.remove(id));
    });
    Runtime::mark_frame_dirty();
    seconds
}

#[must_use]
pub fn running_clocks() -> usize {
    with_sources(|sources| {
        sources
            .map
            .values()
            .filter(|source| matches!(source, FrameSource::Clock(_)))
            .count()
    }) + crate::animation_runtime::state::perpetual_count()
}

pub(crate) fn tick_clocks(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 || running_clocks() == 0 {
        return;
    }
    batch(|| {
        with_sources(|sources| {
            sources.map.retain(|_, source| match source {
                FrameSource::Clock(clock) => {
                    clock.elapsed += f64::from(dt);
                    clock.seconds.try_set(clock.elapsed)
                }
                _ => true,
            });
        });
        Runtime::mark_frame_dirty();
    });
}
