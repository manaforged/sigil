//! W3.1b-B — integration tests for FocusHandle + Context::on_action,
//! dispatch_action, propagate. See DR-046 §4 + §6.
//!
//! Eight scenarios covering real focus + dispatch behaviour:
//! 1. dispatch with empty focus stack — no handler fires.
//! 2. focused handle's handler fires.
//! 3. blur removes the handler from active set.
//! 4. inner focus wins over outer (no propagate by default).
//! 5. `propagate()` walks outward to the next handler.
//! 6. `is_focused` reflects the actual stack.
//! 7. owner cleanup evicts the focus handle on entity drop.
//! 8. re-entrant dispatch (handler dispatches a different action).

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::shadow_unrelated
)]

use criterion as _;
use smallvec as _;
use tokio as _;

use std::cell::Cell;
use std::rc::Rc;

use actions::DeriveAction;
use reactive::{Entity, Owner, Runtime};

#[derive(Clone, DeriveAction)]
struct Ping;

#[derive(Clone, DeriveAction)]
struct Pong;

#[test]
fn dispatch_no_focus_no_handler_fires() {
    Runtime::scope("no_focus", || {
        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        let outer = Entity::<()>::new("outer", |_cx| ());

        outer.update(|_state, cx| {
            let handle = cx.focus_handle();
            cx.action::<Ping>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            // Note: no `cx.focus(&handle)` — handle is not on the stack.
            cx.dispatch(&Ping);
        });

        assert_eq!(
            fired.get(),
            0,
            "handler must not fire when its handle is not on the focus stack"
        );
    });
}

#[test]
fn dispatch_focused_handler_fires() {
    Runtime::scope("focused", || {
        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        let entity = Entity::<()>::new("entity", |_cx| ());

        entity.update(|_state, cx| {
            let handle = cx.focus_handle();
            cx.action::<Ping>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            cx.focus(&handle);
            cx.dispatch(&Ping);
        });

        assert_eq!(fired.get(), 1, "focused handler should fire exactly once");
    });
}

#[test]
fn dispatch_blur_then_handler_skipped() {
    Runtime::scope("blur", || {
        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        let entity = Entity::<()>::new("entity", |_cx| ());

        entity.update(|_state, cx| {
            let handle = cx.focus_handle();
            cx.action::<Ping>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            cx.focus(&handle);
            cx.blur(&handle);
            cx.dispatch(&Ping);
        });

        assert_eq!(fired.get(), 0, "blurred handler must not fire");
    });
}

#[test]
fn dispatch_most_nested_wins_focus_stack() {
    // Two entities, two focus handles, both registered for Ping.
    // Both are pushed to the stack (outer first, inner second). Without
    // `propagate`, only the inner handler should fire.
    Runtime::scope("most_nested_wins", || {
        let outer_fired = Rc::new(Cell::new(0_u32));
        let inner_fired = Rc::new(Cell::new(0_u32));
        let outer_clone = Rc::clone(&outer_fired);
        let inner_clone = Rc::clone(&inner_fired);

        let outer = Entity::<()>::new("outer", |_cx| ());
        let inner = Entity::<()>::new("inner", |_cx| ());

        // Register outer's handler + push outer focus.
        outer.update(|_s, cx| {
            let outer_handle = cx.focus_handle();
            cx.action::<Ping>(&outer_handle, move |_a, _dcx| {
                outer_clone.set(outer_clone.get() + 1);
            });
            cx.focus(&outer_handle);
            // Drop the handle here — focus stack keeps the id alive.
            // Owner-cleanup will evict on scope drop.
            let _ = outer_handle;
        });

        // Register inner's handler + push inner focus + dispatch.
        inner.update(|_s, cx| {
            let inner_handle = cx.focus_handle();
            cx.action::<Ping>(&inner_handle, move |_a, _dcx| {
                inner_clone.set(inner_clone.get() + 1);
            });
            cx.focus(&inner_handle);
            cx.dispatch(&Ping);
            let _ = inner_handle;
        });

        assert_eq!(inner_fired.get(), 1, "inner handler should fire");
        assert_eq!(
            outer_fired.get(),
            0,
            "outer handler must NOT fire without propagate"
        );
    });
}

#[test]
fn dispatch_propagate_walks_outer_focus_stack() {
    // Same shape as `most_nested_wins`, but inner's handler calls
    // `propagate()` — outer should fire too. Order matters: inner
    // fires first, then outer.
    Runtime::scope("propagate", || {
        let order = Rc::new(std::cell::RefCell::new(Vec::<&'static str>::new()));
        let order_inner = Rc::clone(&order);
        let order_outer = Rc::clone(&order);

        let outer = Entity::<()>::new("outer", |_cx| ());
        let inner = Entity::<()>::new("inner", |_cx| ());

        outer.update(|_s, cx| {
            let outer_handle = cx.focus_handle();
            cx.action::<Ping>(&outer_handle, move |_a, _dcx| {
                order_outer.borrow_mut().push("outer");
            });
            cx.focus(&outer_handle);
        });

        inner.update(|_s, cx| {
            let inner_handle = cx.focus_handle();
            cx.action::<Ping>(&inner_handle, move |_a, dcx| {
                order_inner.borrow_mut().push("inner");
                dcx.propagate();
            });
            cx.focus(&inner_handle);
            cx.dispatch(&Ping);
        });

        let trace = order.borrow().clone();
        assert_eq!(
            trace,
            vec!["inner", "outer"],
            "propagate should fire inner first, then outer (leaf-first walk)"
        );
    });
}

#[test]
fn is_focused_reflects_stack() {
    Runtime::scope("is_focused", || {
        let entity = Entity::<()>::new("entity", |_cx| ());

        entity.update(|_s, cx| {
            let handle = cx.focus_handle();
            assert!(
                !cx.focused(&handle),
                "newly minted handle must not be on the stack"
            );
            cx.focus(&handle);
            assert!(cx.focused(&handle), "after focus(), is_focused == true");
            cx.blur(&handle);
            assert!(!cx.focused(&handle), "after blur(), is_focused == false");
        });
    });
}

#[test]
fn focus_handle_cleanup_on_entity_drop() {
    // Mint a focus handle inside an inner Owner scope, push it to the
    // stack, then dispose the owner. The handle's id must be evicted
    // from the focus stack so a subsequent dispatch (with no focus)
    // does not fire the handler that was registered against it.
    Runtime::scope("entity_cleanup", || {
        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        // Nested owner whose disposal will trigger the on_cleanup
        // registered by `focus_handle()`.
        let inner_owner = Owner::new("inner", Owner::current());
        inner_owner.with(|| {
            let entity = Entity::<()>::new("entity", |_cx| ());
            entity.update(|_s, cx| {
                let handle = cx.focus_handle();
                cx.action::<Ping>(&handle, move |_a, _dcx| {
                    fired_clone.set(fired_clone.get() + 1);
                });
                cx.focus(&handle);
                // Confirm focus is live right now.
                assert!(cx.focused(&handle));
            });
        });
        inner_owner.dispose();

        // After the inner owner cascades, the focus handle is gone
        // from the stack. Dispatch a Ping in an unrelated entity with
        // no focus — the handler tied to the disposed handle must not
        // fire.
        let bystander = Entity::<()>::new("bystander", |_cx| ());
        bystander.update(|_s, cx| {
            cx.dispatch(&Ping);
        });

        assert_eq!(
            fired.get(),
            0,
            "handler whose focus handle was cleaned up must not fire on subsequent dispatch"
        );
    });
}

#[test]
fn reentrant_dispatch_works() {
    // Ping's handler re-enters the dispatch path for Pong from inside
    // its own body. The handler closure can't capture `cx`, so it
    // signals via a shared flag and a `proxy` entity's update fires
    // the inner dispatch_action — which is exactly what real consumer
    // code looks like (an action triggers a state mutation that fires
    // another action).
    //
    // Both handlers must fire exactly once on a single top-level
    // dispatch. This proves:
    //  - `ActionRegistry::take_entries::<Ping>` returning while the
    //    `Pong` entries are still installed (different TypeId — no
    //    HashMap conflict).
    //  - The outer Ping dispatch correctly restores `Ping` entries
    //    after the nested Pong dispatch has come and gone.
    Runtime::scope("reentrant", || {
        let ping_fired = Rc::new(Cell::new(0_u32));
        let pong_fired = Rc::new(Cell::new(0_u32));
        let ping_clone = Rc::clone(&ping_fired);
        let pong_clone = Rc::clone(&pong_fired);

        // The `proxy` entity exists solely so the Ping handler has
        // something it can call `update(...)` on to mint a fresh
        // Context for the nested Pong dispatch.
        let proxy = Entity::<()>::new("proxy", |_cx| ());
        let entity = Entity::<()>::new("entity", |_cx| ());

        // Register Pong on the stack.
        entity.update(|_s, cx| {
            let pong_handle = cx.focus_handle();
            cx.action::<Pong>(&pong_handle, move |_a, _dcx| {
                pong_clone.set(pong_clone.get() + 1);
            });
            cx.focus(&pong_handle);
        });

        // Register Ping. Inside Ping's handler, re-enter the dispatch
        // path for Pong via `proxy.update(...)`.
        entity.update(|_s, cx| {
            let ping_handle = cx.focus_handle();
            cx.action::<Ping>(&ping_handle, move |_a, _dcx| {
                ping_clone.set(ping_clone.get() + 1);
                // Re-entrant dispatch — handler body invokes another
                // dispatch_action. With take/restore this works
                // because Ping's entries are taken out, leaving the
                // registry usable for any *other* action type.
                proxy.update(|_s, inner_cx| {
                    inner_cx.dispatch(&Pong);
                });
            });
            cx.focus(&ping_handle);
            cx.dispatch(&Ping);
        });

        assert_eq!(ping_fired.get(), 1, "ping handler fires once");
        assert_eq!(
            pong_fired.get(),
            1,
            "pong handler fires once via re-entrant dispatch from inside ping handler"
        );

        // Second top-level dispatch proves Ping's entries were
        // restored (and not lost) after the re-entrant cycle.
        entity.update(|_s, cx| {
            cx.dispatch(&Ping);
        });
        assert_eq!(ping_fired.get(), 2, "ping fires again after restore");
        // Second dispatch did NOT trigger another Pong (Ping handler
        // increments pong via re-entry; second top-level Ping does
        // the same). Verify count is now 2.
        assert_eq!(
            pong_fired.get(),
            2,
            "pong fires from each re-entrant call inside ping handler"
        );
    });
}
