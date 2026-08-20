//! Windowed showcase for linear-gradient rectangle fills.

#![allow(clippy::missing_panics_doc)]
#![allow(missing_docs)]

#[cfg(windows)]
use windows_core as _;

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_renderer::scenes;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use wgpu as _;

#[path = "support/windowed.rs"]
mod windowed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (scene, viewport, clear) = scenes::gradient();
    windowed::run("Sigil - Gradient Showcase", scene, viewport, clear)
}
