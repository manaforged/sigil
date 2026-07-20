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
mod tests {
    use super::*;
    use sigil_reactive::{Signal, signal};

    #[test]
    fn closure_runs_with_fresh_cx() {
        let mut ran = false;
        with_cx("with-cx-runs", |_cx| {
            ran = true;
        });
        assert!(ran, "closure passed to with_cx must execute");
    }

    #[test]
    fn signal_mutation_inside_scope() {
        with_cx("with-cx-signal", |_cx| {
            let s: Signal<i32> = signal(1);
            s.set(42);
            assert_eq!(s.get(), 42);
        });
    }

    #[test]
    fn with_runtime_runs_closure() {
        let mut ran = false;
        with_runtime("with-runtime-runs", |_| {
            ran = true;
        });
        assert!(ran, "closure passed to with_runtime must execute");
    }

    #[test]
    fn with_runtime_allows_signal_mutation() {
        with_runtime("with-runtime-signal", |_| {
            let s: Signal<i32> = signal(7);
            s.set(99);
            assert_eq!(s.get(), 99);
        });
    }

    #[test]
    fn tree_size_counts_a_single_root() {
        use crate::prelude::container;
        let count = tree_size("tree-size-single", |cx| cx.add(container()));
        assert_eq!(count, 1);
    }

    #[test]
    fn tree_size_counts_children() {
        use crate::prelude::{container, text};
        let count = tree_size("tree-size-children", |cx| {
            cx.add(container().child(text("a")).child(text("b")))
        });
        assert_eq!(count, 3);
    }
}
