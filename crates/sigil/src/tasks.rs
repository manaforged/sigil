#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
#[path = "tasks/lib_tests.rs"]
mod tests;

mod backend;
mod blocking;
pub(crate) mod current;
pub(crate) mod dispatch;
mod handle;
mod message;
mod panic;
mod run;
pub(crate) mod runtime;
mod schedule_error;
mod sleep;
mod status;
mod task;
#[cfg(target_arch = "wasm32")]
mod timer;

pub use backend::TaskWorkers;
pub use blocking::blocking;
pub use current::{installed, signal_writer, spawn};
pub use dispatch::SignalWriter;
pub use handle::OwnerHandle;
pub use message::{MessageReceiver, on_message, on_message_or_close};
pub use panic::panic_message;
pub use run::{Task, task, task_with};
pub use runtime::AsyncRuntime;
pub use schedule_error::ScheduleError;
pub use sleep::sleep;
pub use status::Status;
pub use task::TaskHandle;
