#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
#[path = "tasks/lib_tests.rs"]
mod tests;

mod backend;
mod blocking;
pub(crate) mod current;
pub(crate) mod dispatch;
mod message;
mod panic;
mod run;
pub(crate) mod runtime;
mod sleep;
mod status;
mod task;
#[cfg(target_arch = "wasm32")]
mod timer;

pub(crate) use backend::TaskWorkers;
pub use blocking::blocking;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use current::installed;
pub use current::{signal_writer, spawn};
pub use dispatch::SignalWriter;
pub use message::{MessageReceiver, on_message};
pub(crate) use panic::panic_message;
pub use run::{Task, task};
pub use runtime::AsyncRuntime;
pub use sleep::sleep;
pub use status::Status;
kittest_pub! {
    #[doc(hidden)]
    pub use task::TaskHandle;
}
