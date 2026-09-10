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
