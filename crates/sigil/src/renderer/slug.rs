#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod atlas;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod bands;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod cache;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod curve;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod pipeline;

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod draw;
mod tuning;

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) use draw::GlyphInstance;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub use draw::{DrawParams, SlugTextPipeline};
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) use draw::{QueuedRun, RunFrame, SLUG_INSTANCE_BYTES};
pub use tuning::TextTuning;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub use tuning::{set_text_tuning, text_tuning};
