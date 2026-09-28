#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]
mod atlas;
mod bands;
mod cache;
mod curve;
mod pipeline;

mod draw;
mod tuning;

pub use draw::{DrawParams, SlugTextPipeline};
pub(crate) use draw::{QueuedRun, RunFrame, SLUG_INSTANCE_BYTES};
pub use tuning::{TextTuning, set_text_tuning, text_tuning};
