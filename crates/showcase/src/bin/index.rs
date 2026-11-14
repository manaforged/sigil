//! Runs the index showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    let config = sigil_showcase::window("Index");
    let config = sigil_showcase::index::fonts()
        .into_iter()
        .fold(config, |config, (family, bytes)| config.font(family, bytes, 0));
    sigil::run(config, sigil_showcase::index::app)
}
