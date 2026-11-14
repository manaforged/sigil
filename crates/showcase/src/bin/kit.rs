//! Runs the kit showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(sigil_showcase::window("Kit"), sigil_showcase::kit::app)
}
