//! Tokio-based async runtime and main-thread dispatcher.
//!
//! This crate ships the concrete `Scheduler` adapter promised by
//! [DR-007]: one `tokio::runtime::Runtime` per `App`, an MPSC payload
//! channel drained on the main thread, owner-disposal-driven task
//! cancellation, and the `spawn_to_signal` / `dispatch_to_owner`
//! public entry points from the DR-005 addendum.
//!
//! # Architecture
//!
//! [`AsyncRuntime`] owns the Tokio runtime. Background tasks spawned
//! through it push closures into an [unbounded MPSC][UnboundedSender].
//! Each closure is tagged with an [`OwnerHandle`]; at [drain][Scheduler::drain]
//! time the dispatcher asks `ui-reactive` whether the owner is still
//! alive and either runs the closure or drops it silently. A
//! caller-injected `waker` closure wires the enqueue side to
//! `winit::event_loop::EventLoopProxy::wake_up()` in the real app;
//! tests swap it for a counter or a no-op.
//!
//! Task cancellation is owner-disposal-driven. Every
//! [`AsyncRuntime::spawn`][Scheduler::spawn] call records its
//! `AbortHandle` in a small per-owner registry; on each `drain`, any
//! handles whose owner is no longer live are aborted and dropped.
//! Disposal + next drain therefore cancels the task; no separate
//! cleanup dance is required in user code. See
//! [`spawn_to_signal`] for the high-level wrapper that uses all of
//! this to drive a reactive `Signal<Status<T, E>>`.
//!
//! # What Phase 2 does NOT ship
//!
//! The queue is unbounded in this adapter. Bounded-queue behaviour
//! with `Err(ScheduleError::Full)` surfaced to callers is reserved in
//! the trait surface (`Scheduler::Error`) but not enforced here —
//! we'll bound it once real workloads show a fan-out regression, per
//! DR-007 invariant (5).
//!
//! [DR-007]: ../../../docs/decisions/DR-007-async-tokio.md
//! [UnboundedSender]: tokio::sync::mpsc::UnboundedSender

// `pub(crate)` on items inside private modules expresses design
// intent ("do not leak this even if the module ever becomes pub")
// that clippy's `redundant_pub_crate` nursery lint treats as noise.
// Mirrors the same allow in `ui-reactive`.
#![allow(clippy::redundant_pub_crate)]

// `criterion` is a dev-dep used only by `benches/async_ops.rs`. Cargo
// pulls every dev-dep into the lib-test compilation unit, which trips
// `unused_crate_dependencies` here. Alias silences the lint without
// adding a real import. Same pattern as `ui-reactive`.
#[cfg(test)]
use criterion as _;

mod dispatch;
mod handle;
mod runtime;
mod status;

use winit as _;

pub use dispatch::{dispatch_to_owner, spawn_to_signal};
pub use handle::OwnerHandle;
pub use runtime::{AsyncRuntime, Dispatcher, ScheduleError};
pub use status::Status;
