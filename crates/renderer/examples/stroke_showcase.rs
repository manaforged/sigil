#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Windowed showcase for rectangle strokes.

#![allow(clippy::missing_panics_doc)]
#![allow(missing_docs)]

#[cfg(feature = "windowed")]
use sigil_renderer::scenes;

#[cfg(feature = "windowed")]
#[path = "support/windowed.rs"]
mod windowed;

#[cfg(feature = "windowed")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (scene, viewport, clear) = scenes::stroke();
    windowed::run("Sigil - Stroke Showcase", scene, viewport, clear)
}

#[cfg(not(feature = "windowed"))]
fn main() {}
