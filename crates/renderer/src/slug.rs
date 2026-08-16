//! Slug GPU text rendering pipeline.
//!
//! Implements the Lengyel 2017 band-based fragment-shader rasteriser.
//! Patent US 10,373,352 entered the public domain on 2026-03-17.

mod atlas;
mod bands;
mod cache;
mod curve;
mod pipeline;

mod draw;
mod tuning;

pub(crate) use draw::SLUG_INSTANCE_BYTES;
pub use draw::{DrawParams, SlugTextPipeline};
pub use tuning::{TextTuning, set_text_tuning, text_tuning};
