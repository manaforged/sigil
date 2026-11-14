//! Runs the beam showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::window("Beam"),
        sigil_showcase::beam::app,
    )
}
