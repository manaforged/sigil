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
    let [(base, _, clear), (overlay, _, _)] = scenes::screen_split();
    windowed::run_split(
        "Sigil - Split Path Showcase",
        [base, overlay],
        (1280.0, 400.0),
        clear,
    )
}

#[cfg(not(feature = "windowed"))]
fn main() {}
