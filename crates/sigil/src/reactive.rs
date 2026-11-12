#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

mod arena;
mod batch;
#[cfg(event_channel)]
mod channel;
kittest_pub! {
    /// Diagnostics for the reactive graph.
    #[doc(hidden)]
    pub mod debug;
}
mod diagnostic;
mod effect;
mod effect_run;
pub(crate) mod frame;
mod interval;
mod memo;
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
#[cfg(event_channel)]
pub use channel::{EventChannel, Subscription};
pub(crate) use effect::effect_at;
#[cfg(not(feature = "kittest"))]
pub(crate) use effect::effect_named;
pub(crate) use effect::{current_effect_owner, run_dispatch_deferred};
pub use effect::{defer, effect, untrack};
pub(crate) use frame::tick_frame;
#[cfg(feature = "kittest")]
pub(crate) use interval::tick_intervals;
#[cfg(feature = "test-clock")]
pub use interval::{__test_advance_clock, __test_clear_clock, __test_set_clock};
pub use interval::{Timer, interval, timeout};
pub use memo::memo;
kittest_pub! {
    #[cfg(host)]
    #[doc(hidden)]
    pub use interval::next_interval_deadline;
}
pub use owner::{Owner, cleanup};
pub(crate) use owner::{is_live, owner_by_id};
kittest_pub! {
    #[doc(hidden)]
    pub use phase::DrawPhase;
}
kittest_pub! {
    #[doc(hidden)]
    pub use runtime::Runtime;
}
pub(crate) use signal::SendableSignal;
pub use signal::{Signal, signal};
pub use source::Reactive;

#[cfg(feature = "kittest")]
pub use trace::test_hook as __trace_test_hook;

#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use effect::effect_named;
