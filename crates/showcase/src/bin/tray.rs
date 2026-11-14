//! Runs the tray showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Tray").size(sigil_showcase::SIZE[0], sigil_showcase::SIZE[1]),
        sigil_showcase::tray::app,
    )
}
