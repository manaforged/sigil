#[cfg(host)]
mod atlas;
#[cfg(host)]
mod bands;
#[cfg(host)]
mod cache;
#[cfg(host)]
mod curve;
#[cfg(host)]
mod pipeline;

#[cfg(host)]
mod draw;
mod tuning;

#[cfg(host)]
pub(crate) use draw::GlyphInstance;
#[cfg(host)]
pub use draw::{DrawParams, SlugTextPipeline};
#[cfg(host)]
pub(crate) use draw::{QueuedRun, RunFrame, SLUG_INSTANCE_BYTES};
pub use tuning::TextTuning;
#[cfg(host)]
pub use tuning::{set_text_tuning, text_tuning};
