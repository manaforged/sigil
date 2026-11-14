//! Runs the console showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::figure("Console", sigil_showcase::sheet::fonts()),
        sigil_showcase::console::app,
    )
}
