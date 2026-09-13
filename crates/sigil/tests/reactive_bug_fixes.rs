#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic)]

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use sigil::reactive::{Owner, Runtime, Signal, batch, effect, on_cleanup, signal};

#[test]
fn cleanup_inside_effect_fires_before_next_run() {
    Runtime::scope("cleanup_inside_effect_fires_before_next_run", || {
        let trigger = signal(0_i32);
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let log_for_effect = Rc::clone(&log);
        effect(move || {
            let _ = trigger.get();
            let log_for_cleanup = Rc::clone(&log_for_effect);
            on_cleanup(move || log_for_cleanup.borrow_mut().push("cleanup"));
            log_for_effect.borrow_mut().push("run");
        });
        assert_eq!(*log.borrow(), vec!["run"]);

        trigger.set(1);
        assert_eq!(*log.borrow(), vec!["run", "cleanup", "run"]);
    });
}

#[test]
fn signal_inside_effect_is_disposed_on_re_run() {
    Runtime::scope("signal_inside_effect_is_disposed_on_re_run", || {
        let trigger = signal(0_i32);
        let captured: Rc<Cell<Option<Signal<i32>>>> = Rc::new(Cell::new(None));
        let captured_for = Rc::clone(&captured);
        effect(move || {
            let _ = trigger.get();
            let inner = signal(42_i32);
            captured_for.set(Some(inner));
        });
        let first = captured.get().expect("effect ran once");

        trigger.set(1);

        let probe = catch_unwind(AssertUnwindSafe(|| {
            let _ = first.peek();
        }));
        assert!(
            probe.is_err(),
            "signal created inside previous effect run must be disposed on re-run"
        );
    });
}

#[test]
fn nested_effect_inside_outer_effect_is_disposed_on_outer_re_run() {
    Runtime::scope(
        "nested_effect_inside_outer_effect_is_disposed_on_outer_re_run",
        || {
            let outer_trigger = signal(0_i32);
            let inner_trigger = signal(0_i32);
            let inner_runs = Rc::new(Cell::new(0_u32));
            let inner_runs_outer = Rc::clone(&inner_runs);

            effect(move || {
                let _ = outer_trigger.get();
                let inner_runs_clone = Rc::clone(&inner_runs_outer);
                effect(move || {
                    let _ = inner_trigger.get();
                    inner_runs_clone.set(inner_runs_clone.get() + 1);
                });
            });
            assert_eq!(inner_runs.get(), 1);

            inner_trigger.set(1);
            assert_eq!(inner_runs.get(), 2);

            outer_trigger.set(1);
            assert_eq!(inner_runs.get(), 3);

            inner_trigger.set(2);
            assert_eq!(
                inner_runs.get(),
                4,
                "old inner effect must be disposed; only the new one runs"
            );
        },
    );
}

#[test]
fn owner_current_inside_effect_is_distinct_from_outer_scope() {
    Runtime::scope("owner_current_inside_effect_is_distinct", || {
        let outside = Owner::current().expect("inside Runtime::scope");
        let captured_inside: Rc<Cell<Option<_>>> = Rc::new(Cell::new(None));
        let captured_for = Rc::clone(&captured_inside);
        effect(move || {
            captured_for.set(Owner::current());
        });
        let inside = captured_inside.get().expect("effect ran");
        assert_ne!(
            outside, inside,
            "effect body runs with its own run-owner, distinct from the enclosing scope's owner"
        );
    });
}

#[test]
fn panic_inside_owner_with_restores_prior_current_owner() {
    Runtime::scope("panic_inside_owner_with", || {
        let outer = Owner::current().expect("scope active");
        let child = Owner::new("child", Owner::current());
        let result = catch_unwind(AssertUnwindSafe(|| {
            child.with(|| {
                panic!("boom");
            });
        }));
        assert!(result.is_err());
        assert_eq!(Owner::current(), Some(outer));
        child.dispose();
    });
}

#[test]
fn panic_inside_batch_decrements_depth() {
    Runtime::scope("panic_inside_batch", || {
        let first = catch_unwind(AssertUnwindSafe(|| {
            batch(|| {
                panic!("boom");
            });
        }));
        assert!(first.is_err());

        let n = signal(0_i32);
        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || {
                let _ = n.get();
                runs.set(runs.get() + 1);
            });
        }
        assert_eq!(runs.get(), 1);
        batch(|| {
            n.set(1);
            n.set(2);
            assert_eq!(runs.get(), 1);
        });
        assert_eq!(runs.get(), 2);
    });
}

#[test]
fn panic_inside_effect_clears_running_set() {
    Runtime::scope("panic_inside_effect_clears_running_set", || {
        let n = signal(0_i32);
        let first_done = Rc::new(Cell::new(false));
        {
            let first_done = Rc::clone(&first_done);
            let n_for = n;
            effect(move || {
                let _ = n_for.get();
                assert!(!first_done.get(), "boom");
                first_done.set(true);
            });
        }

        let second_runs = Rc::new(Cell::new(0_u32));
        {
            let second_runs = Rc::clone(&second_runs);
            let n_for = n;
            effect(move || {
                let _ = n_for.get();
                second_runs.set(second_runs.get() + 1);
            });
        }
        assert_eq!(second_runs.get(), 1);

        let result = catch_unwind(AssertUnwindSafe(|| {
            n.set(1);
        }));
        assert!(result.is_err(), "first effect panics through the write");

        let pre = second_runs.get();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            n.set(2);
        }));
        assert!(
            second_runs.get() > pre,
            "second effect must be able to re-run after the first's panic"
        );
    });
}

#[test]
fn panic_inside_update_restores_value_and_clears_sentinel() {
    Runtime::scope("panic_inside_update", || {
        let n = signal(7_i32);
        let result = catch_unwind(AssertUnwindSafe(|| {
            n.update(|v| {
                *v = 99;
                panic!("boom");
            });
        }));
        assert!(result.is_err());

        assert_eq!(n.peek(), 99, "partial mutation is preserved");
        n.set(1);
        assert_eq!(n.peek(), 1);
    });
}
