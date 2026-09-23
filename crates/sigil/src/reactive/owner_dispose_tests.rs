#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::manual_assert,
    clippy::shadow_unrelated
)]

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use crate::reactive::Runtime;
use crate::reactive::batch::batch;
use crate::reactive::effect::effect_named;
use crate::reactive::owner::{Owner, is_live, on_cleanup};
use crate::reactive::signal::signal;

#[test]
fn queued_effect_on_disposed_signal_panics_contained() {
    Runtime::scope("owner_dispose::queued_dead_dep", || {
        let sig_owner = Owner::new("sig", Owner::current());
        let fx_owner = Owner::new("fx", Owner::current());
        let mark = sig_owner.with(|| signal(0_u32));
        let fired = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&fired);
        fx_owner.with(move || {
            effect_named("test::reader", move || {
                let _ = mark.get();
                writer.set(writer.get() + 1);
            });
        });
        assert_eq!(fired.get(), 1);
        let result = catch_unwind(AssertUnwindSafe(|| {
            batch(|| {
                mark.set(1);
                sig_owner.dispose();
            });
        }));
        assert!(
            result.is_err(),
            "a queued effect that reads a disposed dep must panic"
        );
        let (name, _location) =
            Runtime::panic_origin_info().expect("panic origin names the reader");
        assert_eq!(name, "test::reader");
        Runtime::recover_from_panic();
        fx_owner.dispose();
    });
}

#[test]
fn conditional_dep_effect_survives_an_unread_disposal() {
    Runtime::scope("owner_dispose::conditional_dep", || {
        let sig_owner = Owner::new("sig", Owner::current());
        let fx_owner = Owner::new("fx", Owner::current());
        let mark = sig_owner.with(|| signal(0_u32));
        let gate = fx_owner.with(|| signal(true));
        let fired = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&fired);
        fx_owner.with(move || {
            effect_named("test::conditional", move || {
                let _value = if gate.get() { mark.get() } else { 0 };
                writer.set(writer.get() + 1);
            });
        });
        assert_eq!(fired.get(), 1);
        sig_owner.dispose();
        gate.set(false);
        assert_eq!(
            fired.get(),
            2,
            "an effect that does not read its dead dep keeps running"
        );
        let result = catch_unwind(AssertUnwindSafe(|| gate.set(true)));
        assert!(
            result.is_err(),
            "reading the disposed dep on a later run must panic"
        );
        let (name, _location) =
            Runtime::panic_origin_info().expect("panic origin names the reader");
        assert_eq!(name, "test::conditional");
        Runtime::recover_from_panic();
        fx_owner.dispose();
    });
}

#[test]
fn effect_disposing_its_ancestor_is_reaped_after_the_run() {
    Runtime::scope("owner_dispose::ancestor", || {
        let host = Owner::new("host", Owner::current());
        let tick = host.with(|| signal(0_u32));
        let outer = Owner::new("outer", Owner::current());
        let fired = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&fired);
        outer.with(move || {
            effect_named("test::self_dispose", move || {
                let _ = tick.get();
                writer.set(writer.get() + 1);
                if writer.get() == 2 {
                    outer.dispose();
                }
            });
        });
        tick.set(1);
        assert_eq!(fired.get(), 2, "the disposing run completes");
        assert!(!is_live(outer.id()), "ancestor owner is gone after the run");
        tick.set(2);
        assert_eq!(fired.get(), 2, "the reaped effect never runs again");
        host.dispose();
    });
}

#[test]
fn cleanups_skipped_during_unwind_run_at_recovery() {
    Runtime::scope("owner_dispose::deferred_cleanup", || {
        let owner = Owner::new("victim", Owner::current());
        let ran = Rc::new(Cell::new(false));
        let flag = Rc::clone(&ran);
        let result = catch_unwind(AssertUnwindSafe(|| {
            owner.with(move || {
                effect_named("test::cleanup", move || {
                    on_cleanup({
                        let flag = Rc::clone(&flag);
                        move || flag.set(true)
                    });
                    panic!("boom");
                });
            });
        }));
        assert!(result.is_err(), "the panicking effect unwinds");
        assert!(
            !ran.get(),
            "cleanup bodies must not run while the thread is unwinding"
        );
        Runtime::recover_from_panic();
        assert!(ran.get(), "deferred cleanup runs at panic recovery");
        owner.dispose();
    });
}

#[test]
fn recover_from_panic_during_a_run_keeps_the_running_effect() {
    Runtime::scope("owner_dispose::recover_in_run", || {
        let owner = Owner::new("o", Owner::current());
        let probe = Rc::new(Cell::new(0_u32));
        let p = Rc::clone(&probe);
        let me = owner;
        owner.with(move || {
            effect_named("test::recover_in_run", move || {
                me.dispose();
                Runtime::recover_from_panic();
                let _created = signal(1_u32);
                p.set(p.get() + 1);
            });
        });
        assert_eq!(
            probe.get(),
            1,
            "the run completes after a mid-run recover_from_panic"
        );
    });
}
