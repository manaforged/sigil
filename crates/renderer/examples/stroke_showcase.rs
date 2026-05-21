//! Windowed showcase for rectangle strokes.

#![allow(clippy::missing_panics_doc)]
#![allow(missing_docs)]

#[cfg(windows)]
use windows_core as _;

use ahash as _;
use bytemuck as _;
use criterion as _;
use hal as _;
use half as _;
use icons as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use renderer::scenes;
use shell as _;
use skrifa as _;
use smallvec as _;
use style as _;
use text as _;
use tracing as _;
use wgpu as _;

#[path = "support/windowed.rs"]
mod windowed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (scene, viewport, clear) = scenes::stroke();
    windowed::run("Sigil - Stroke Showcase", scene, viewport, clear)
}
