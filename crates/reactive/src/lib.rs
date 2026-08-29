#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil as _;

mod arena;
mod async_cx;
mod batch;
mod channel;
mod cx;
mod cx_actions;
mod cx_focus;
mod cx_spawn;
mod cx_spawn_local;
pub mod debug;
mod derived;
mod diagnostic;
mod effect;
mod effect_run;
mod entity;
mod entity_global;
mod entity_observer;
mod focus;
mod interval;
mod interval_registry;
mod local_executor;
mod owner;
mod owner_dispose;
mod phase;
mod runtime;
mod runtime_drain;
mod runtime_executor;
mod runtime_phase;
mod runtime_slots;
mod signal;
mod signal_ops;
mod signal_write;
mod task;
mod task_local;
mod trace;

#[doc(hidden)]
pub use async_cx::{
    AsyncContext, BoxMainCallback, BoxSendFuture, DispatchError, MainThreadDispatcher, TaskExecutor,
};
pub use batch::batch;
pub use channel::EventChannel;
#[doc(hidden)]
pub use cx::EntityCx;
pub use derived::{derived, derived_eq};
pub use effect::{effect, effect_named};
#[doc(hidden)]
pub use entity::{Entity, EntityId};
#[doc(hidden)]
pub use entity_global::{global, global_mut, observe_global, set_global};
pub use entity_observer::Subscription;
#[doc(hidden)]
pub use focus::{FocusHandle, FocusHandleId};
#[doc(hidden)]
pub use interval::{__test_advance_clock, __test_clear_clock, __test_set_clock};
pub use interval::{
    IntervalId, cancel_interval, next_interval_deadline, register_interval, tick_intervals,
};
#[doc(hidden)]
pub use local_executor::{BoxLocalFuture, LocalTaskExecutor};
pub use owner::{Owner, OwnerId, is_live, on_cleanup, owner_by_id};
pub use phase::DrawPhase;
pub use runtime::Runtime;
#[doc(hidden)]
pub use signal::SendableSignal;
pub use signal::{Signal, signal};
#[doc(hidden)]
pub use task::{EntityGone, Task, TaskAwaitError};
#[doc(hidden)]
pub use task_local::LocalTask;

#[doc(hidden)]
pub use trace::test_hook as __trace_test_hook;
