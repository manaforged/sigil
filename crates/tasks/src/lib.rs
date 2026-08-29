#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

mod current;
mod dispatch;
mod handle;
mod reactive_bridge;
mod runtime;
mod schedule_error;
mod status;

pub use current::{dispatcher, post, spawn};
pub use dispatch::dispatch_to_owner;
#[doc(hidden)]
pub use dispatch::spawn_to_signal;
pub use handle::OwnerHandle;
pub use reactive_bridge::{ReactiveBridge, TokioExecutor};
pub use runtime::{AsyncRuntime, Dispatcher};
pub use schedule_error::ScheduleError;
pub use status::Status;
