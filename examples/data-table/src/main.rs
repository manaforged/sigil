//! `data-table` — 1:1 Sigil window of GPUI 0.2.2 `examples/data_table.rs`.
//!
//! Same 10 000 quotes, 24 columns, zebra rows, green/red last-done
//! tint, status `Total N items, visible range: start..end`, and a
//! 1280×1000 window. Body is `virtual_list_bind` so it matches
//! GPUI `uniform_list` (windowed rows, not `table()`).
//!
//! ```text
//! cargo run --release -p data-table
//! SIGIL_PROFILE=1 cargo run --profile profiling -p data-table
//! cargo run --release --example data_table --manifest-path \
//!   "${CARGO_HOME:-$HOME/.cargo}/registry/src/index.crates.io-*/gpui-0.2.2/Cargo.toml"
//! ```

use data_table::view::app_tree;
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - data-table", 1280, 1000), build_root)
}

fn build_root(cx: &mut Cx) -> NodeId {
    cx.add(app_tree())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
