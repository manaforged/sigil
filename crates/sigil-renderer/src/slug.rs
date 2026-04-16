//! Slug GPU text rendering pipeline.
//!
//! Implements the Lengyel 2017 band-based fragment-shader rasteriser.
//! Patent US 10,373,352 entered the public domain on 2026-03-17.

mod bands;
mod cache;
mod curve;
mod pipeline;

mod draw;

pub use draw::{DrawParams, SlugTextPipeline};
