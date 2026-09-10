#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic)]

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use sigil_reactive::{Owner, Runtime, Signal, batch, effect, signal};

#[test]
fn owner_new_with_disposed_parent_panics() {
    Runtime::scope("owner_new_disposed_parent", || {
        let parent = Owner::new("doomed", Owner::current());
        let stale_id = parent.id();
        parent.dispose();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = Owner::new("orphan", Some(stale_id));
        }));
        assert!(
            result.is_err(),
            "creating an owner under a disposed parent must panic"
        );
    });
}

#[test]
fn owner_with_on_disposed_owner_panics() {
    Runtime::scope("owner_with_disposed", || {
        let owner = Owner::new("temp", Owner::current());
        let alias = owner;
        owner.dispose();
        let result = catch_unwind(AssertUnwindSafe(|| {
            alias.with(|| {});
        }));
        assert!(
            result.is_err(),
            "Owner::with on a disposed owner must panic"
        );
    });
}

#[test]
fn signal_after_active_owner_disposed_mid_flight_panics() {
    Runtime::scope("signal_disposed_active_owner", || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            Runtime::scope("inner_root", || {
                let parent = Owner::new("parent", Owner::current());
                parent.with(|| {
                    parent.dispose();
                    let _ = signal(0_i32);
                });
            });
        }));
        assert!(
            result.is_err(),
            "signal() with a disposed current_owner must panic"
        );
    });
}

#[test]
fn signal_set_with_dropping_t_does_not_deadlock_runtime() {
    Runtime::scope("set_drop_touches_runtime", || {
        struct Dropper {
            witness: Rc<Cell<bool>>,
            touch: Signal<i32>,
        }
        impl Drop for Dropper {
            fn drop(&mut self) {
                self.witness.set(true);
                self.touch.set(self.touch.peek() + 1);
            }
        }
        impl PartialEq for Dropper {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.witness, &other.witness)
            }
        }

        let witness = Rc::new(Cell::new(false));
        let touch = signal(0_i32);
        let holder: Signal<Option<Dropper>> = signal(Some(Dropper {
            witness: Rc::clone(&witness),
            touch,
        }));

        holder.set(None);

        assert!(witness.get(), "old value was dropped");
        assert_eq!(touch.peek(), 1, "drop's runtime touch succeeded");
    });
}

#[test]
fn effect_writing_during_drain_does_not_run_twice() {
    Runtime::scope("drain_ordering", || {
        let trigger = signal(0_i32);
        let bridge = signal(0_i32);

        effect(move || {
            let v = trigger.get();
            bridge.set(v * 10);
        });

        let b_runs = Rc::new(Cell::new(0_u32));
        {
            let b_runs = Rc::clone(&b_runs);
            effect(move || {
                let _ = bridge.get();
                b_runs.set(b_runs.get() + 1);
            });
        }
        assert_eq!(b_runs.get(), 1);

        batch(|| {
            trigger.set(3);
        });

        assert_eq!(
            b_runs.get(),
            2,
            "effect B must run exactly once per outer-batch exit even when \
             effect A writes its subscribed signal during the drain"
        );
        assert_eq!(bridge.peek(), 30);
    });
}
