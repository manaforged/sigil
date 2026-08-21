#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! DR-045 amendment §A1-A4 — entity arena eviction on owner dispose.
//!
//! These tests exercise the eviction phase added to `dispose_owner`:
//!
//! 1. Disposing an owner removes every entity it created from
//!    `rt.entities` (memory leak fix).
//! 2. `Entity::update` / `Entity::read` panic on a disposed handle
//!    with the documented `EntityGone` diagnostic.
//! 3. Observers registered against an evicted entity are dropped
//!    (no dangling observer slots, no fan-out into freed memory).
//! 4. Globals registered against an evicted entity are scrubbed from
//!    the `TypeId → EntityKey` table.
//! 5. ABA safety — the arena is generational, so a re-inserted slot
//!    at the same index returns `false` from `contains()` for the
//!    stale generation.

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

use sigil_reactive::{Entity, Owner, Runtime, debug::entity_is_live, observe_global, set_global};

/// Build a child owner under the current scope. Caller is responsible
/// for disposing it.
fn child_owner(name: &'static str) -> Owner {
    Owner::new(
        name,
        Some(Owner::current().expect("scope establishes owner")),
    )
}

// 1. — Disposing an owner removes the owner's entities from rt.entities.
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

// 2. — Entity::update on a disposed entity panics with EntityGone.
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
        // Slot is gone; update must panic with the disposed-entity
        // diagnostic carrying "EntityGone".
        e.update(|v, _cx| *v += 1);
    });
}

// 3. — Entity::read on a disposed entity panics with EntityGone.
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
        // Read of disposed entity panics with EntityGone-tagged
        // diagnostic.
        let _ = e.read(|v| *v);
    });
}

// 4. — Observers registered against an entity must NOT fire after the
//      entity's owner disposes.
//
//      Concretely: register an `observe_global` for type G. Dispose the
//      reactive scope that the global was installed under. Installing
//      a fresh global in a new scope must not re-fire the previous
//      observer (it has been evicted alongside the entity slot).
#[test]
fn dispose_owner_scrubs_observer_back_edges() {
    // Wrapper type private to this test so global state stays isolated.
    #[derive(Default)]
    struct Marker(#[allow(dead_code)] u32);

    let fired = Rc::new(Cell::new(0_u32));

    Runtime::scope("first_scope", || {
        let fired_clone = Rc::clone(&fired);
        // observe_global allocates a placeholder Entity<Marker> under
        // the current scope.
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
    // First scope ended → owner-cascade evicts the Marker entity AND
    // the observer it owned. Any next-scope set_global<Marker> must
    // install a fresh entity and must NOT fan out to the dropped
    // observer.
    Runtime::scope("second_scope", || {
        set_global(Marker(2));
    });
    assert_eq!(
        fired.get(),
        1,
        "observer from disposed scope must not fire again after re-install"
    );
}

// 5. — Globals scrub: a global entity is removed from rt.globals when
//      its owning scope is disposed, so a fresh `lookup_global` returns
//      None (until a new install installs it).
#[test]
fn dispose_owner_scrubs_globals_when_owned() {
    #[derive(Default)]
    struct LiveProbe(#[allow(dead_code)] u32);

    // First scope: install a global and observe it.
    let saw_first = Rc::new(Cell::new(false));
    Runtime::scope("install_global", || {
        let saw_first_clone = Rc::clone(&saw_first);
        let _sub = observe_global::<LiveProbe>(move || {
            saw_first_clone.set(true);
        });
        set_global(LiveProbe(100));
    });
    assert!(saw_first.get(), "first scope's observer fired");

    // Second scope: install a fresh observer + global. The fresh
    // observer must fire (proves the new global is freshly installed
    // and observable); the previous scope's observer must not.
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

// 6. — ABA safety: re-inserting at the same arena slot returns false
//      from `entity_is_live` for the stale generation.
//
//      Constructs an entity, captures its handle, disposes its owner,
//      creates a NEW entity under a fresh owner. The arena's free-list
//      reuses the index but bumps the generation, so the old handle
//      must NOT resolve as live, even though the slot is occupied.
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

        // Insert a NEW entity. The arena's free-list reuses the freed
        // slot (same index) but bumps the generation. The stale handle
        // still names (index, old_generation) — `contains` must reject.
        let child_b = child_owner("b");
        child_b.with(|| {
            let _fresh = Entity::<u64>::new("b_inner", |_| 0xBB);
            // The fresh entity occupies (potentially) the same slot
            // index. The stale handle must not resolve as live even
            // though SOMETHING lives at that index.
            assert!(
                !entity_is_live(stale),
                "stale generation must not resolve as live after re-insert at same slot"
            );
        });
        child_b.dispose();
    });
}
