//! Headless test helpers.
//!
//! Wraps the two-step ceremony every smoke test repeats: enter a
//! reactive [`Runtime`] scope, then construct a fresh root [`Cx`].
//! See DR-049 / dogfood-gap-closure plan Phase B for the rationale.

use crate::prelude::{Cx, NodeId};
use sigil_reactive::Runtime;

/// Run a closure inside a fresh [`Cx`] and reactive scope. Convenient
/// for tree-build smoke tests that need to construct signals.
///
/// ```no_run
/// use sigil::testing::with_cx;
///
/// with_cx("my-app-smoke", |cx| {
///     // build a tree, mutate signals, walk the kernel — anything
///     // that needs a live reactive runtime + root Cx.
///     let _ = cx;
/// });
/// ```
pub fn with_cx<F: FnOnce(&mut Cx)>(name: &'static str, f: F) {
    Runtime::scope(name, || {
        let mut cx = Cx::new_root();
        f(&mut cx);
    });
}

/// Run a closure inside a fresh reactive [`Runtime`] scope.
///
/// Lighter than [`with_cx`] — useful for tests that mutate signals
/// outside a Cx-bound tree (e.g. asserting a derived value tracks its
/// inputs without building a node tree first).
///
/// ```no_run
/// use sigil::testing::with_runtime;
/// use sigil::reactive::{signal, Signal};
///
/// with_runtime("derived-tracks-inputs", |_| {
///     let s: Signal<i32> = signal(1);
///     s.set(2);
///     assert_eq!(s.get(), 2);
/// });
/// ```
pub fn with_runtime<F: FnOnce(&Runtime)>(name: &'static str, f: F) {
    Runtime::scope(name, || f(&Runtime));
}

/// Build a tree via `f` inside a fresh [`Cx`] + reactive scope and
/// return the node count.
///
/// One-line collapse of the `with_cx` + `visit_preorder` + counter
/// boilerplate that every smoke test was writing by hand.
///
/// ```no_run
/// use sigil::prelude::*;
/// use sigil::testing::tree_size;
///
/// let count = tree_size("my-smoke", |cx| {
///     cx.add(container().child(text("hi")))
/// });
/// assert!(count >= 2);
/// ```
pub fn tree_size(name: &'static str, f: impl FnOnce(&mut Cx) -> NodeId) -> usize {
    let mut size = 0;
    with_cx(name, |cx| {
        let root = f(cx);
        cx.visit_preorder(root, |_, _| size += 1);
    });
    size
}

#[cfg(test)]
#[path = "testing_tests.rs"]
mod tests;
