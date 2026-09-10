#![allow(
    unused_crate_dependencies,
    reason = "headless harness dev-dependencies are used by the library tests only"
)]

use std::path::PathBuf;

use data_table::TOTAL_ITEMS;
use ledger::{App, app_tree, bench};
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--bench") {
        let out = args
            .next()
            .map_or_else(|| PathBuf::from("ledger-sigil.json"), PathBuf::from);
        return bench::run(out);
    }
    sigil::launch(("Sigil - ledger", 1280, 1000), |cx: &mut Cx| {
        let app = App::new(TOTAL_ITEMS);
        cx.add(app_tree(cx, &app))
    })
}
