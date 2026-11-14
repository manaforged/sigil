//! Runs the deck showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(sigil_showcase::window("Deck"), sigil_showcase::deck::app)
}
