#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Integration tests for `#[derive(Action)]`.
//!
//! Two real action types: a unit struct and a payload-bearing struct.
//! Each test asserts the observable surface that the derive must
//! generate: `Action::debug_name` returns the type's identifier and
//! the type compile-time satisfies the `Action` trait bound.

#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use std::cell::Cell;
use std::rc::Rc;

use sigil_actions::{Action, ActionRegistry, HandlerKey};
use sigil_actions_macros::Action;

#[derive(Clone, Action)]
struct CloseTab;

#[derive(Clone, Action)]
struct GoToLine {
    pub id: u32,
}

const fn assert_impls<T: Action>() {}

#[test]
fn derive_unit_struct_yields_type_name() {
    let action = CloseTab;
    assert_eq!(
        action.debug_name(),
        "CloseTab",
        "debug_name() must return the struct identifier literal"
    );
    assert_impls::<CloseTab>();
}

#[test]
fn derive_payload_struct_yields_type_name_and_keeps_field() {
    let action = GoToLine { id: 42 };
    assert_eq!(action.debug_name(), "GoToLine");
    // The payload field must survive — derive may not strip data.
    assert_eq!(action.id, 42);
    assert_impls::<GoToLine>();
}

#[test]
fn derive_boxed_clone_preserves_payload_and_dispatches() {
    // The derive emits `boxed_clone` that delegates to Clone::clone.
    // Verify two observable properties:
    //  1. The boxed payload's debug_name matches the original (so the
    //     dyn-dispatch routing keys on the same TypeId).
    //  2. Driving the boxed clone through ActionRegistry::dispatch_dyn
    //     fires a handler registered against the original type and
    //     delivers the original payload value.
    let original = GoToLine { id: 7 };
    let cloned: Box<dyn Action> = original.boxed_clone();
    assert_eq!(
        cloned.debug_name(),
        "GoToLine",
        "boxed_clone preserves the type's debug name"
    );

    let mut registry = ActionRegistry::new();
    let received = Rc::new(Cell::new(0_u32));
    let received_clone = Rc::clone(&received);
    registry.register::<GoToLine>(HandlerKey::from_raw(1), move |a: &GoToLine, _| {
        received_clone.set(a.id);
    });

    let result = registry.dispatch_dyn(cloned.as_ref(), &[HandlerKey::from_raw(1)]);
    assert!(
        result.handled(),
        "dispatch_dyn on a derive(Action) boxed_clone must fire the registered handler"
    );
    assert_eq!(
        received.get(),
        7,
        "boxed_clone preserves the payload value through the dyn-dispatch round-trip"
    );
}
