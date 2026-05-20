//! Verifies the W4.99.C1 prelude trims + `Action` / `DispatchCx` /
//! `dispatch_to_owner` promotions.
//!
//! This is a compile-only test — if the items resolve through
//! `sigil::prelude::*`, the trims and promotions landed correctly.

// The test crate inherits sigil's dev-deps + deps but only names items
// via `sigil::prelude::*`. Suppress the `unused_crate_dependencies`
// avalanche that the dep set produces here.
#![allow(dead_code, unused_imports, unused_crate_dependencies)]

use sigil::prelude::*;

// Action surface promoted from sigil-actions.
#[derive(Clone, DeriveAction)]
struct PreludeProbeAction;

fn _action_in_prelude() -> Box<dyn Action> {
    Box::new(PreludeProbeAction)
}

const fn _dispatch_cx_in_prelude(cx: &mut DispatchCx) {
    cx.propagate();
}

const fn _handler_key_in_prelude() -> HandlerKey {
    HandlerKey::from_raw(1)
}

// Async dispatch helper promoted from sigil-async. Signature-only —
// `dispatch_to_owner` is generic over `F: FnOnce() + Send + 'static`
// and takes a `&Dispatcher`, so we just confirm the symbol resolves
// by referencing it as a path expression below.
#[test]
fn dispatch_to_owner_resolves_through_prelude() {
    // Path-expression reference; we don't invoke it (would need a live
    // dispatcher), only prove the symbol is reachable.
    let f: fn(&_, OwnerHandle, fn()) = dispatch_to_owner::<fn()>;
    let _ = f;
}
