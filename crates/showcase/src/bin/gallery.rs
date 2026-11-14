//! Runs the gallery showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::window("Gallery"),
        sigil_showcase::gallery::app,
    )
}
