#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::rc::Rc;

use super::owner_by_id;
use crate::reactive::Owner;

#[test]
fn owner_by_id_runs_inside_a_live_owner_and_skips_a_disposed_one() {
    Owner::scope("owner_by_id", || {
        let owner = Owner::new("view", Owner::current());
        let inside = Rc::new(Cell::new(None));
        let seen = Rc::clone(&inside);
        assert!(owner_by_id(owner, move || seen.set(Owner::current())));
        assert_eq!(inside.get(), Some(owner));
        assert_ne!(Owner::current(), Some(owner));

        owner.dispose();
        let called = Rc::new(Cell::new(false));
        let flag = Rc::clone(&called);
        assert!(!owner_by_id(owner, move || flag.set(true)));
        assert!(!called.get());
        assert!(!owner.is_alive());
    });
}
