#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

mod arena;
mod batch;
mod channel;
pub mod debug;
mod derived;
mod diagnostic;
mod effect;
mod effect_run;
mod interval;
mod interval_registry;
mod owner;
mod owner_dispose;
mod phase;
mod runtime;
mod runtime_phase;
mod runtime_slots;
mod signal;
mod signal_ops;
mod signal_write;
mod source;
mod trace;

pub use batch::batch;
pub use channel::{EventChannel, Subscription};
pub use derived::derived;
pub(crate) use effect::{current_effect_owner, run_dispatch_deferred};
pub use effect::{defer, effect, effect_named, untrack};
pub(crate) use interval::tick_intervals;
#[cfg(any(test, feature = "test-clock"))]
pub use interval::{__test_advance_clock, __test_clear_clock, __test_set_clock};
pub use interval::{IntervalId, cancel_interval, interval, next_interval_deadline};
pub use owner::{Owner, OwnerId, on_cleanup};
pub(crate) use owner::{is_live, owner_by_id};
pub use phase::DrawPhase;
pub use runtime::Runtime;
pub(crate) use signal::SendableSignal;
pub use signal::{Signal, signal};
pub use source::Reactive;

#[cfg(any(test, feature = "kittest"))]
pub use trace::test_hook as __trace_test_hook;
