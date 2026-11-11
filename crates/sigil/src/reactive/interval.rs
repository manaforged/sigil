use core::time::Duration;
use slotmap::Key;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use web_time::Instant;

use crate::reactive::frame::{FrameSource, SourceId, Sources, try_with_sources, with_sources};
use crate::reactive::owner::{Owner, on_cleanup, owner_by_id};
use crate::reactive::runtime::Runtime;

/// Stops the interval or timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalId(pub(crate) SourceId);

pub(crate) struct Deadline {
    seq: u64,
    owner: Owner,
    period: Option<Duration>,
    next_fire: Instant,
    callback: Box<dyn Fn()>,
}

/// Runs a closure on a repeating timer.
#[track_caller]
pub fn interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId {
    register(period, Some(period), Box::new(callback))
}

/// Run `callback` once after `delay`.
#[track_caller]
pub fn timeout(delay: Duration, callback: impl FnOnce() + 'static) -> IntervalId {
    let once = RefCell::new(Some(callback));
    let fire = move || {
        let taken = once.borrow_mut().take();
        if let Some(run) = taken {
            run();
        }
    };
    register(delay, None, Box::new(fire))
}

#[track_caller]
fn register(delay: Duration, period: Option<Duration>, callback: Box<dyn Fn()>) -> IntervalId {
    let owner = Owner::current().expect("interval or timeout called outside a reactive scope");
    let id = with_sources(|sources| {
        let deadline = Deadline {
            seq: sources.next_seq(),
            owner,
            period,
            next_fire: sources.now() + delay,
            callback,
        };
        IntervalId(sources.map.insert(FrameSource::Deadline(deadline)))
    });
    on_cleanup(move || id.cancel());
    id
}

impl IntervalId {
    /// Stops the interval or timeout.
    pub fn cancel(self) {
        let removed = try_with_sources(|sources| sources.map.remove(self.0));
        drop(removed);
    }
}

fn deadline_mut(sources: &mut Sources, id: SourceId) -> Option<&mut Deadline> {
    match sources.map.get_mut(id) {
        Some(FrameSource::Deadline(deadline)) => Some(deadline),
        _ => None,
    }
}

#[must_use = "shells can ignore the count; tests use it to assert that fires actually happened"]
pub(crate) fn tick_intervals() -> usize {
    let (now, mut due) = with_sources(|sources| {
        let now = sources.now();
        let due: Vec<(u64, SourceId, Owner)> = sources
            .map
            .iter()
            .filter_map(|(id, source)| match source {
                FrameSource::Deadline(deadline) if deadline.next_fire <= now => {
                    Some((deadline.seq, id, deadline.owner))
                }
                _ => None,
            })
            .collect();
        (now, due)
    });
    due.sort_unstable_by_key(|(seq, _, _)| *seq);
    let mut fired = 0;
    for (_, id, owner) in due {
        if invoke(id, owner) {
            fired += 1;
        }
        let removed = with_sources(|sources| reschedule(sources, id, now));
        drop(removed);
    }
    fired
}

fn reschedule(sources: &mut Sources, id: SourceId, now: Instant) -> Option<FrameSource> {
    let deadline = deadline_mut(sources, id)?;
    match deadline.period {
        Some(period) => {
            deadline.next_fire = now + period;
            None
        }
        None => sources.map.remove(id),
    }
}

fn invoke(id: SourceId, owner: Owner) -> bool {
    let Some(callback) = with_sources(|sources| {
        let stub: Box<dyn Fn()> = Box::new(|| {});
        deadline_mut(sources, id).map(|deadline| core::mem::replace(&mut deadline.callback, stub))
    }) else {
        return false;
    };
    let ran =
        catch_unwind(AssertUnwindSafe(|| owner_by_id(owner, &*callback))).unwrap_or_else(|_| {
            eprintln!(
                "[sigil-reactive] interval callback panicked (id={}); \
             slot restored, callback will re-run on next tick",
                id.data().as_ffi()
            );
            Runtime::recover_after_swallowed_panic();
            true
        });
    with_sources(|sources| {
        if let Some(deadline) = deadline_mut(sources, id) {
            deadline.callback = callback;
        }
    });
    ran
}

kittest_pub! {
    /// Earliest next fire time of the registered intervals and timeouts.
    #[must_use]
    #[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
    pub fn next_interval_deadline() -> Option<Instant> {
        with_sources(|sources| {
            sources
                .map
                .values()
                .filter_map(|source| match source {
                    FrameSource::Deadline(deadline) => Some(deadline.next_fire),
                    _ => None,
                })
                .min()
        })
    }
}

/// Sets the interval clock.
#[cfg(any(test, feature = "test-clock"))]
pub fn __test_set_clock(clock: Instant) {
    with_sources(|sources| sources.test_clock = Some(clock));
}

/// Moves the interval clock forward by `delta`.
#[cfg(any(test, feature = "test-clock"))]
pub fn __test_advance_clock(delta: Duration) {
    with_sources(|sources| sources.test_clock = Some(sources.now() + delta));
}

/// Returns the interval clock to system time.
#[cfg(any(test, feature = "test-clock"))]
pub fn __test_clear_clock() {
    with_sources(|sources| sources.test_clock = None);
}

#[cfg(test)]
#[path = "interval_tests.rs"]
mod tests;
