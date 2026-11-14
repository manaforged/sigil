//! Runs the proximity field figure in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        sigil_showcase::figure("Field", sigil_showcase::sheet::fonts()),
        sigil_showcase::field::app,
    )
}
