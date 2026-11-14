//! Runs the deck showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Deck").size(sigil_showcase::SIZE[0], sigil_showcase::SIZE[1]),
        sigil_showcase::deck::app,
    )
}
