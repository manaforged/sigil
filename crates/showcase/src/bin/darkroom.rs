//! Runs the darkroom showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Darkroom").size(1180, 720),
        sigil_showcase::darkroom::app,
    )
}
