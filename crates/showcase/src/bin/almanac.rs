//! Runs the almanac showcase in a window.

use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(
        AppConfig::new("Almanac").size(1180, 720),
        sigil_showcase::almanac::app,
    )
}
