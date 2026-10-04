#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::rc::Rc;

use crate::reactive::{Owner, Runtime};
use crate::tasks::OwnerHandle;

#[test]
fn run_calls_inside_a_live_owner_and_skips_a_disposed_one() {
    Runtime::scope("tasks::owner_handle_run", || {
        let owner = Owner::new("view", Owner::current());
        let handle = OwnerHandle::from_id(owner.id());
        let inside = Rc::new(Cell::new(None));
        let seen = Rc::clone(&inside);
        assert!(handle.run(move || seen.set(Owner::current())));
        assert_eq!(inside.get(), Some(owner.id()));
        assert_ne!(Owner::current(), Some(owner.id()));

        owner.dispose();
        let called = Rc::new(Cell::new(false));
        let flag = Rc::clone(&called);
        assert!(!handle.run(move || flag.set(true)));
        assert!(!called.get());
        assert!(!handle.is_alive());
    });
}
