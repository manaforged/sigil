//! Runs the index showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::figure(
            "Index",
            sigil_showcase::sheet::fonts()
                .into_iter()
                .chain(sigil_showcase::index::fonts()),
        ),
        sigil_showcase::index::app,
    )
}
