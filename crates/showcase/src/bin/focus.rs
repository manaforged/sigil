//! Runs the focus showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Focus").size(sigil_showcase::SIZE[0], sigil_showcase::SIZE[1]),
        sigil_showcase::focus::app,
    )
}
