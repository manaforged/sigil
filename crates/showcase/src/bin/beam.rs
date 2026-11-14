//! Runs the beam showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Beam").size(sigil_showcase::SIZE[0], sigil_showcase::SIZE[1]),
        sigil_showcase::beam::app,
    )
}
