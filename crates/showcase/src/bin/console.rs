//! Runs the console showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::window("Console"),
        sigil_showcase::console::app,
    )
}
