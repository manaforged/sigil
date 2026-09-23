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
mod trace;

pub use batch::batch;
pub use channel::{EventChannel, Subscription};
pub use derived::{derived, derived_eq};
pub(crate) use effect::current_effect_owner;
pub use effect::{effect, effect_named};
#[doc(hidden)]
pub use interval::{__test_advance_clock, __test_clear_clock, __test_set_clock};
pub use interval::{
    IntervalId, cancel_interval, next_interval_deadline, register_interval, tick_intervals,
};
pub use owner::{Owner, OwnerId, is_live, on_cleanup, owner_by_id};
pub use phase::DrawPhase;
pub use runtime::Runtime;
#[doc(hidden)]
pub use signal::SendableSignal;
pub use signal::{Signal, signal};

#[doc(hidden)]
pub use trace::test_hook as __trace_test_hook;
