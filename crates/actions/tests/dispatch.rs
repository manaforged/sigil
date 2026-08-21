#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Integration tests for `ActionRegistry` dispatch semantics.
//!
//! Every test asserts observable behaviour — handler side-effects on a
//! `Cell`, fire-ordering, payload delivery — not tautological
//! roundtrips. See DR-046 §4 and §6 for the contract being verified.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::similar_names
)]

use std::cell::Cell;
use std::rc::Rc;

use std::any::Any;

use sigil_actions::{Action, ActionRegistry, HandlerKey};

#[derive(Clone)]
struct Ping;
impl Action for Ping {
    fn debug_name(&self) -> &'static str {
        "Ping"
    }
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
struct Pong;
impl Action for Pong {
    fn debug_name(&self) -> &'static str {
        "Pong"
    }
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
struct CountBy(u32);
impl Action for CountBy {
    fn debug_name(&self) -> &'static str {
        "CountBy"
    }
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn dispatch_no_handlers_returns_unhandled() {
    let mut registry = ActionRegistry::new();
    let result = registry.dispatch(&Ping, &[HandlerKey::from_raw(1)]);
    assert!(!result.handled(), "empty registry must report unhandled");
    assert!(
        result.fired_owners().is_empty(),
        "no owners may appear in fired list for an unhandled dispatch"
    );
}

#[test]
fn dispatch_single_handler_fires() {
    let mut registry = ActionRegistry::new();
    let counter = Rc::new(Cell::new(0_u32));
    let c2 = Rc::clone(&counter);
    registry.register::<Ping>(HandlerKey::from_raw(1), move |_, _| {
        c2.set(c2.get() + 1);
    });

    let result = registry.dispatch(&Ping, &[HandlerKey::from_raw(1)]);

    assert!(result.handled());
    assert_eq!(result.fired_owners(), &[HandlerKey::from_raw(1)]);
    assert_eq!(
        counter.get(),
        1,
        "handler body must have executed exactly once"
    );
}

#[test]
fn dispatch_most_nested_wins() {
    let mut registry = ActionRegistry::new();
    let outer_fired = Rc::new(Cell::new(false));
    let inner_fired = Rc::new(Cell::new(false));
    {
        let of = Rc::clone(&outer_fired);
        registry.register::<Ping>(HandlerKey::from_raw(1), move |_, _| {
            of.set(true);
        });
    }
    {
        let inf = Rc::clone(&inner_fired);
        registry.register::<Ping>(HandlerKey::from_raw(2), move |_, _| {
            inf.set(true);
        });
    }

    // Chain is [outer=1, inner=2] — most-nested first means key 2 fires.
    let result = registry.dispatch(&Ping, &[HandlerKey::from_raw(1), HandlerKey::from_raw(2)]);

    assert!(result.handled());
    assert_eq!(
        result.fired_owners(),
        &[HandlerKey::from_raw(2)],
        "only the most-nested handler should fire by default"
    );
    assert!(inner_fired.get(), "inner handler must have fired");
    assert!(
        !outer_fired.get(),
        "outer handler must NOT fire without propagate()"
    );
}

#[test]
fn dispatch_propagate_walks_outer() {
    let mut registry = ActionRegistry::new();
    let outer_fired = Rc::new(Cell::new(false));
    let inner_fired = Rc::new(Cell::new(false));
    {
        let of = Rc::clone(&outer_fired);
        registry.register::<Ping>(HandlerKey::from_raw(1), move |_, _| {
            of.set(true);
        });
    }
    {
        let inf = Rc::clone(&inner_fired);
        registry.register::<Ping>(HandlerKey::from_raw(2), move |_, cx| {
            inf.set(true);
            cx.propagate();
        });
    }

    let result = registry.dispatch(&Ping, &[HandlerKey::from_raw(1), HandlerKey::from_raw(2)]);

    assert!(result.handled());
    assert_eq!(
        result.fired_owners(),
        &[HandlerKey::from_raw(2), HandlerKey::from_raw(1)],
        "fire order must be most-nested first, then outer after propagate()"
    );
    assert!(inner_fired.get());
    assert!(outer_fired.get(), "outer handler must fire after propagate");
}

#[test]
fn dispatch_unhandled_outside_chain() {
    let mut registry = ActionRegistry::new();
    let fired = Rc::new(Cell::new(false));
    let f2 = Rc::clone(&fired);
    registry.register::<Ping>(HandlerKey::from_raw(1), move |_, _| {
        f2.set(true);
    });

    // Handler is on key 1; chain only walks key 2.
    let result = registry.dispatch(&Ping, &[HandlerKey::from_raw(2)]);

    assert!(
        !result.handled(),
        "handler outside the dispatch chain must not fire"
    );
    assert!(result.fired_owners().is_empty());
    assert!(!fired.get(), "handler body must not have executed");
}

#[test]
fn dispatch_type_keyed_isolation() {
    let mut registry = ActionRegistry::new();
    let ping_fired = Rc::new(Cell::new(0_u32));
    let pong_fired = Rc::new(Cell::new(0_u32));
    {
        let pf = Rc::clone(&ping_fired);
        registry.register::<Ping>(HandlerKey::from_raw(1), move |_, _| {
            pf.set(pf.get() + 1);
        });
    }
    {
        let pf = Rc::clone(&pong_fired);
        registry.register::<Pong>(HandlerKey::from_raw(1), move |_, _| {
            pf.set(pf.get() + 1);
        });
    }

    let result = registry.dispatch(&Ping, &[HandlerKey::from_raw(1)]);

    assert!(result.handled());
    assert_eq!(ping_fired.get(), 1, "Ping handler fires for Ping dispatch");
    assert_eq!(
        pong_fired.get(),
        0,
        "Pong handler must NOT fire when Ping was dispatched"
    );
}

#[test]
fn dispatch_passes_action_payload() {
    let mut registry = ActionRegistry::new();
    let received = Rc::new(Cell::new(0_u32));
    let r2 = Rc::clone(&received);
    registry.register::<CountBy>(HandlerKey::from_raw(1), move |action: &CountBy, _| {
        r2.set(action.0);
    });

    let result = registry.dispatch(&CountBy(42), &[HandlerKey::from_raw(1)]);

    assert!(result.handled());
    assert_eq!(
        received.get(),
        42,
        "payload must survive Box<dyn Any> downcast on the dispatch path"
    );
}
