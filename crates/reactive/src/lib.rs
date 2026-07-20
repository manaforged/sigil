//! Reactive core — signals, effects, owner tree.
//!
//! See [`DR-005`] for the full design. Short version: a Leptos 0.5+
//! style owner tree, arena-backed signals and effects, main-thread
//! only, with `#[track_caller]` location capture and named owners
//! for debuggability.
//!
//! # Example
//!
//! ```rust,ignore
//! use sigil_reactive::{effect, signal, Runtime};
//!
//! Runtime::scope("test::counter", || {
//!     let count = signal(0_i32);
//!     let seen = std::cell::Cell::new(0);
//!     effect(move || {
//!         seen.set(count.get());
//!     });
//!     assert_eq!(seen.get(), 0);
//!     count.set(7);
//!     assert_eq!(seen.get(), 7);
//! });
//! ```
//!
//! (Doctests are marked `ignore` while we work around a Rust 1.94
//! merged-doctests target-detection bug on Windows-msvc. The same
//! examples are exercised end-to-end in
//! `tests/reactive_contract.rs`.)
//!
//! [`DR-005`]: ../../../docs/decisions/DR-005-reactivity-owner-tree.md

#![deny(missing_docs)]
// `pub(crate)` on items inside private modules expresses design
// intent ("do not leak this even if the module ever becomes pub")
// that clippy's `redundant_pub_crate` nursery lint treats as noise.
// We keep the explicit restriction.
#![allow(clippy::redundant_pub_crate)]

// `criterion` is a dev-dep used only by the `reactive_ops` bench.
// Cargo's lib-test target pulls all dev-deps into the compilation
// unit, which trips `unused_crate_dependencies` here. Alias silences
// the lint without adding a real import.
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil as _;

mod arena;
mod async_cx;
mod batch;
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
mod entity_a11y_observer;
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
mod trigger;

pub use async_cx::{
    AsyncContext, BoxMainCallback, BoxSendFuture, DispatchError, MainThreadDispatcher, TaskExecutor,
};
pub use batch::batch;
pub use cx::Context;
pub use derived::{derived, derived_eq};
pub use effect::{effect, effect_named};
pub use entity::{Entity, EntityId};
pub use entity_a11y_observer::A11yObserver;
pub use entity_global::{global, global_mut, observe_global, set_global};
pub use entity_observer::Subscription;
pub use focus::{FocusHandle, FocusHandleId};
pub use interval::{
    __test_advance_clock, __test_clear_clock, __test_set_clock, IntervalId, cancel_interval,
    next_interval_deadline, register_interval, tick_intervals,
};
pub use local_executor::{BoxLocalFuture, LocalTaskExecutor};
pub use owner::{Owner, OwnerId, is_live, on_cleanup, owner_by_id};
pub use phase::DrawPhase;
pub use runtime::Runtime;
pub use signal::{SendableSignal, Signal, signal};
pub use task::{EntityGone, Task, TaskAwaitError};
pub use task_local::LocalTask;
pub use trigger::{EventChannel, Trigger};

// Re-exported for integration tests only. The `test_hook` submodule
// is `#[doc(hidden)]` and carries no stability promise. See the
// module docs for the effect trace log format.
#[doc(hidden)]
pub use trace::test_hook as __trace_test_hook;
