#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_const_for_fn
)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::reactive::{Entity, Owner, Runtime, debug::entity_is_live, observe_global, set_global};

fn child_owner(name: &'static str) -> Owner {
    Owner::new(
        name,
        Some(Owner::current().expect("scope establishes owner")),
    )
}

#[test]
fn dispose_owner_evicts_entities_from_arena() {
    Runtime::scope("evict_arena", || {
        let child = child_owner("child");
        let captured: Cell<Option<Entity<i32>>> = Cell::new(None);
        child.with(|| {
            let e = Entity::<i32>::new("counter", |_| 41);
            captured.set(Some(e));
        });
        let e = captured.get().expect("entity built inside child scope");
        assert!(entity_is_live(e), "entity is live before owner disposes");
        child.dispose();
        assert!(
            !entity_is_live(e),
            "entity slot must be removed from rt.entities after owner dispose"
        );
    });
}

#[test]
#[should_panic(expected = "EntityGone")]
fn update_after_dispose_panics_with_diagnostic() {
    Runtime::scope("update_after_dispose", || {
        let child = child_owner("child");
        let mut captured: Option<Entity<i32>> = None;
        child.with(|| {
            captured = Some(Entity::<i32>::new("counter", |_| 0));
        });
        let e = captured.expect("entity built inside child scope");
        child.dispose();
        e.update(|v, _cx| *v += 1);
    });
}

#[test]
#[should_panic(expected = "EntityGone")]
fn read_after_dispose_panics_with_diagnostic() {
    Runtime::scope("read_after_dispose", || {
        let child = child_owner("child");
        let mut captured: Option<Entity<i32>> = None;
        child.with(|| {
            captured = Some(Entity::<i32>::new("counter", |_| 17));
        });
        let e = captured.expect("entity built inside child scope");
        child.dispose();
        let _ = e.read(|v| *v);
    });
}

#[test]
fn dispose_owner_scrubs_observer_back_edges() {
    #[derive(Default)]
    struct Marker(#[allow(dead_code)] u32);

    let fired = Rc::new(Cell::new(0_u32));

    Runtime::scope("first_scope", || {
        let fired_clone = Rc::clone(&fired);
        let _sub = observe_global::<Marker>(move || {
            fired_clone.set(fired_clone.get() + 1);
        });
        set_global(Marker(1));
        assert_eq!(
            fired.get(),
            1,
            "observer fires while its owner is still alive"
        );
    });
    Runtime::scope("second_scope", || {
        set_global(Marker(2));
    });
    assert_eq!(
        fired.get(),
        1,
        "observer from disposed scope must not fire again after re-install"
    );
}

#[test]
fn dispose_owner_scrubs_globals_when_owned() {
    #[derive(Default)]
    struct LiveProbe(#[allow(dead_code)] u32);

    let saw_first = Rc::new(Cell::new(false));
    Runtime::scope("install_global", || {
        let saw_first_clone = Rc::clone(&saw_first);
        let _sub = observe_global::<LiveProbe>(move || {
            saw_first_clone.set(true);
        });
        set_global(LiveProbe(100));
    });
    assert!(saw_first.get(), "first scope's observer fired");

    let saw_second = Rc::new(Cell::new(false));
    Runtime::scope("reinstall_global", || {
        let saw_second_clone = Rc::clone(&saw_second);
        let _sub = observe_global::<LiveProbe>(move || {
            saw_second_clone.set(true);
        });
        set_global(LiveProbe(200));
    });
    assert!(
        saw_second.get(),
        "fresh observer in new scope fires (proves a brand-new entity was \
         registered, not the stale evicted one)"
    );
}

#[test]
fn aba_safety_re_insert_at_same_slot() {
    Runtime::scope("aba_safety", || {
        let child_a = child_owner("a");
        let mut handle_a: Option<Entity<u64>> = None;
        child_a.with(|| {
            handle_a = Some(Entity::<u64>::new("a_inner", |_| 0xAA));
        });
        let stale = handle_a.expect("entity built in child_a");
        assert!(entity_is_live(stale), "fresh entity is live");
        child_a.dispose();
        assert!(
            !entity_is_live(stale),
            "evicted entity is not live before re-insert"
        );

        let child_b = child_owner("b");
        child_b.with(|| {
            let _fresh = Entity::<u64>::new("b_inner", |_| 0xBB);
            assert!(
                !entity_is_live(stale),
                "stale generation must not resolve as live after re-insert at same slot"
            );
        });
        child_b.dispose();
    });
}
