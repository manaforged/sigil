//! Runs the focus showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(sigil_showcase::window("Focus"), sigil_showcase::focus::app)
}
