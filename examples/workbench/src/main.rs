#![allow(missing_docs)]

mod compose;
mod diffs;
mod icons;
mod pages;
mod sidebar;
mod state;
mod thread;

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

use sigil::prelude::*;

use crate::sidebar::rail;
use crate::state::Shell;
use crate::thread::main_col;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil — Workbench", 1180, 780), app_tree)
}

pub(crate) fn app_tree(cx: &mut Cx) -> NodeId {
    let shell = Shell::seed();
    cx.add(root(shell))
}

pub(crate) fn root(shell: Shell) -> NodeBuilder {
    container()
        .role(Role::Application)
        .label("Workbench")
        .layout(LayoutStyle::flex_row().fill())
        .class(classes!("bg-gray-1"))
        .child(rail(shell))
        .child(main_col(shell))
}
