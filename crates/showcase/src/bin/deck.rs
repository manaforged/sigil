//! Runs the deck showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Deck").size(1180, 720),
        sigil_showcase::deck::app,
    )
}
