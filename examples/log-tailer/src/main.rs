//! `log-tailer` — a dogfooded viewer-class app on Sigil's `virtual_list`.
//!
//! Synthesises ~10k mock log lines and renders them through the virtual
//! list with a text filter, a level filter, an auto-scroll toggle, and
//! row selection. The corpus never changes — this is strictly a
//! viewer; nothing is written back to a log.

mod data;
mod state;
mod view;

use sigil::prelude::*;

use crate::state::AppState;
use crate::view::app_tree;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - log-tailer", 960, 640), build_root)
}

fn build_root(cx: &mut Cx) -> NodeId {
    let state = AppState::new();
    cx.add(app_tree(state))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sigil::testing::with_cx;

    #[test]
    fn tree_builds_without_panic() {
        with_cx("log-tailer-smoke", |cx| {
            let state = AppState::new();
            let root = cx.add(app_tree(state));
            // NodeId is non-null by construction; assert via child count.
            assert!(cx.child_count(root) > 0, "root must have children");
        });
    }
}
