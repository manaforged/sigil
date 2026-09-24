#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
#[path = "tasks/lib_tests.rs"]
mod tests;

mod current;
mod dispatch;
mod handle;
mod message;
mod panic;
mod runtime;
mod schedule_error;
mod sleep;
mod status;

pub use current::{dispatcher, installed, post, signal_writer, spawn};
pub use dispatch::spawn_to_signal;
pub use dispatch::{SignalWriter, dispatch_to_owner};
pub use handle::OwnerHandle;
pub use message::{MessageReceiver, on_message, on_message_or_close};
pub use panic::panic_message;
pub use runtime::{AsyncRuntime, Dispatcher};
pub use schedule_error::ScheduleError;
pub use sleep::sleep;
pub use status::Status;
