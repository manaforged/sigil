//! Runs the almanac showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::window("Almanac"),
        sigil_showcase::almanac::app,
    )
}
