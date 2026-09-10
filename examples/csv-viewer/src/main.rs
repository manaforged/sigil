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
