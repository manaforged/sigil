#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! DR-015 footgun guards on `NodeBuilder::focusable(true)` for fragments.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil_kernel::{container, fragment};

#[test]
fn focusable_false_on_fragment_is_allowed() {
    let _ = fragment().focusable(false);
}

#[test]
fn focusable_true_on_container_is_allowed() {
    let _ = container().focusable(true);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "focusable(true) on a Fragment")]
fn focusable_true_on_fragment_panics_in_debug() {
    let _ = fragment().focusable(true);
}
