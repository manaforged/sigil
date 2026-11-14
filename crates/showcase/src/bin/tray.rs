//! Runs the tray showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::window("Tray"),
        sigil_showcase::tray::app,
    )
}
