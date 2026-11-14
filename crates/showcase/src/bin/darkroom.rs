//! Runs the darkroom showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::window("Darkroom"),
        sigil_showcase::darkroom::app,
    )
}
