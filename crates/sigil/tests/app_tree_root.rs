#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use sigil::kernel::{Cx, container, text};
use sigil::reactive::{Runtime, Signal, signal};

#[test]
fn a_component_closure_builds_its_node() {
    Runtime::scope("component-builds", || {
        let mut cx = Cx::new_root();
        let root = cx.add(|| container().child(text("hi")));
        assert!(cx.get(root).is_some());
    });
}

#[test]
fn a_component_owns_the_signals_it_creates() {
    Runtime::scope("component-owns", || {
        let made: Rc<Cell<Option<Signal<i32>>>> = Rc::new(Cell::new(None));
        let slot = Rc::clone(&made);
        let mut cx = Cx::new_root();
        let root = cx.add(move || {
            slot.set(Some(signal(7)));
            container()
        });
        let state = made.get().unwrap();
        assert_eq!(state.peek(), 7);
        cx.remove(root);
        assert!(
            catch_unwind(AssertUnwindSafe(|| state.peek())).is_err(),
            "a component's signal outlived its node"
        );
    });
}
