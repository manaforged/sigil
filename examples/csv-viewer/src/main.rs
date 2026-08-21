//! `csv-viewer` — a dogfooded viewer-class app on Sigil's `virtual_list`.
//!
//! Synthesises a ~5k-row mock CSV (six columns of trade-ticket-shaped
//! data) and renders it as a virtualised table: a fixed header row at
//! the top, the body scrolls beneath it, click a row to highlight it.
//! The corpus never changes — this is strictly a viewer; nothing is
//! written back.

mod data;
mod state;
mod view;

use sigil::prelude::*;

use crate::state::AppState;
use crate::view::app_tree;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - csv-viewer", 960, 640), build_root)
}

fn build_root(cx: &mut Cx) -> NodeId {
    let state = AppState::new();
    cx.add(app_tree(&state))
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
