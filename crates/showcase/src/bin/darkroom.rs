//! Runs the darkroom showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Darkroom").size(sigil_showcase::SIZE[0], sigil_showcase::SIZE[1]),
        sigil_showcase::darkroom::app,
    )
}
