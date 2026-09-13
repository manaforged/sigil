#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::shadow_unrelated
)]

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use sigil::actions::Action;
use sigil::reactive::{Entity, Owner, Runtime};

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
    Runtime::scope("most_nested_wins", || {
        let outer_fired = Rc::new(Cell::new(0_u32));
        let inner_fired = Rc::new(Cell::new(0_u32));
        let outer_clone = Rc::clone(&outer_fired);
        let inner_clone = Rc::clone(&inner_fired);

        let outer = Entity::<()>::new("outer", |_cx| ());
        let inner = Entity::<()>::new("inner", |_cx| ());

        outer.update(|_s, cx| {
            let outer_handle = cx.focus_handle();
            cx.action::<Ping>(&outer_handle, move |_a, _dcx| {
                outer_clone.set(outer_clone.get() + 1);
            });
            cx.focus(&outer_handle);
            let _ = outer_handle;
        });

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
    Runtime::scope("entity_cleanup", || {
        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        let inner_owner = Owner::new("inner", Owner::current());
        inner_owner.with(|| {
            let entity = Entity::<()>::new("entity", |_cx| ());
            entity.update(|_s, cx| {
                let handle = cx.focus_handle();
                cx.action::<Ping>(&handle, move |_a, _dcx| {
                    fired_clone.set(fired_clone.get() + 1);
                });
                cx.focus(&handle);
                assert!(cx.focused(&handle));
            });
        });
        inner_owner.dispose();

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
    Runtime::scope("reentrant", || {
        let ping_fired = Rc::new(Cell::new(0_u32));
        let pong_fired = Rc::new(Cell::new(0_u32));
        let ping_clone = Rc::clone(&ping_fired);
        let pong_clone = Rc::clone(&pong_fired);

        let proxy = Entity::<()>::new("proxy", |_cx| ());
        let entity = Entity::<()>::new("entity", |_cx| ());

        entity.update(|_s, cx| {
            let pong_handle = cx.focus_handle();
            cx.action::<Pong>(&pong_handle, move |_a, _dcx| {
                pong_clone.set(pong_clone.get() + 1);
            });
            cx.focus(&pong_handle);
        });

        entity.update(|_s, cx| {
            let ping_handle = cx.focus_handle();
            cx.action::<Ping>(&ping_handle, move |_a, _dcx| {
                ping_clone.set(ping_clone.get() + 1);
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

        entity.update(|_s, cx| {
            cx.dispatch(&Ping);
        });
        assert_eq!(ping_fired.get(), 2, "ping fires again after restore");
        assert_eq!(
            pong_fired.get(),
            2,
            "pong fires from each re-entrant call inside ping handler"
        );
    });
}
