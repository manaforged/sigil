#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::missing_panics_doc)]
#![allow(missing_docs)]

#[cfg(feature = "windowed")]
use sigil_renderer::scenes;

#[cfg(feature = "windowed")]
#[path = "support/windowed.rs"]
mod windowed;

#[cfg(feature = "windowed")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (scene, viewport, clear) = scenes::gradient();
    windowed::run("Sigil - Gradient Showcase", scene, viewport, clear)
}

#[cfg(not(feature = "windowed"))]
fn main() {}
