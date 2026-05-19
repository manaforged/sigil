//! Edge-case behaviour tests beyond the DR-005 happy paths.
//!
//! Each test asserts a specific contract that is easy to break by
//! accident in future changes: dependency tracking under control
//! flow, batched coalescing under nesting, disposal cascades across
//! multiple owner levels, type isolation, re-entrant updates, and
//! the safe / unsafe boundary for calling reactive ops inside a
//! `Signal::update` closure.

#![allow(missing_docs)]

// Silence `unused_crate_dependencies` — criterion is a dev-dep for
// benches, smallvec a regular dep for subscriber storage; neither is
// referenced in this test file.
use criterion as _;
use sigil_actions as _;
use smallvec as _;
use tokio as _;

use std::cell::Cell;
use std::rc::Rc;

use sigil_reactive::{Owner, Runtime, batch, effect, on_cleanup, signal};

// ----- subscription mechanics -----

#[test]
fn multiple_subscribers_to_same_signal_all_fire() {
    Runtime::scope("multi_sub", || {
        let n = signal(0_i32);
        let a = Rc::new(Cell::new(-1));
        let b = Rc::new(Cell::new(-1));
        {
            let a = Rc::clone(&a);
            effect(move || a.set(n.get()));
        }
        {
            let b = Rc::clone(&b);
            effect(move || b.set(n.get() * 2));
        }
        n.set(5);
        assert_eq!(a.get(), 5);
        assert_eq!(b.get(), 10);
    });
}

#[test]
fn diamond_dependency_propagates_through_intermediate_signal() {
    // Source signal feeds a derived-style intermediate (an effect
    // that writes another signal), which in turn feeds a sink.
    // Once derived() lands in Phase 1.1b, this same shape becomes
    // the canonical diamond test.
    Runtime::scope("diamond", || {
        let src = signal(1_i32);
        let mid = signal(0_i32);
        let out = Rc::new(Cell::new(-1));

        effect(move || mid.set(src.get() * 10));
        {
            let out = Rc::clone(&out);
            effect(move || out.set(mid.get()));
        }
        assert_eq!(out.get(), 10);

        src.set(7);
        assert_eq!(out.get(), 70);
    });
}

#[test]
fn conditional_dependencies_are_cleared_between_runs() {
    Runtime::scope("conditional_deps", || {
        let cond = signal(true);
        let a = signal(1_i32);
        let b = signal(100_i32);
        let last = Rc::new(Cell::new(0));
        {
            let last = Rc::clone(&last);
            effect(move || {
                let v = if cond.get() { a.get() } else { b.get() };
                last.set(v);
            });
        }
        assert_eq!(last.get(), 1);

        // b is not currently subscribed — write should be a no-op.
        b.set(200);
        assert_eq!(last.get(), 1);

        // a is subscribed — write should fire the effect.
        a.set(2);
        assert_eq!(last.get(), 2);

        // Flip cond, re-runs the effect with b's branch active.
        cond.set(false);
        assert_eq!(last.get(), 200);

        // Now a should be inactive.
        a.set(99);
        assert_eq!(last.get(), 200);

        // And b should be active.
        b.set(300);
        assert_eq!(last.get(), 300);
    });
}

#[test]
fn effect_with_zero_deps_runs_once_and_never_again() {
    Runtime::scope("zero_dep", || {
        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || runs.set(runs.get() + 1));
        }
        assert_eq!(runs.get(), 1);

        // Unrelated signal write should not re-run the effect.
        let n = signal(0_i32);
        n.set(1);
        n.set(2);
        assert_eq!(runs.get(), 1);
    });
}

#[test]
fn write_to_signal_with_no_subscribers_is_safe() {
    Runtime::scope("orphan_write", || {
        let n = signal(0_i32);
        n.set(1);
        n.set(2);
        n.update(|v| *v += 100);
        assert_eq!(n.get(), 102);
    });
}

#[test]
fn signals_of_different_types_dont_conflate() {
    Runtime::scope("type_disjoint", || {
        let i = signal(1_i32);
        let s = signal(String::from("hi"));
        let f = signal(1.5_f64);

        assert_eq!(i.get(), 1);
        assert_eq!(s.get(), "hi");
        assert!((f.get() - 1.5).abs() < f64::EPSILON);

        i.set(42);
        s.set("hello".to_string());
        f.set(7.25);

        assert_eq!(i.get(), 42);
        assert_eq!(s.get(), "hello");
        assert!((f.get() - 7.25).abs() < f64::EPSILON);
    });
}

#[test]
fn set_with_same_value_still_fires_effects() {
    // We do not equality-check writes. Memoization is the caller's
    // job (or `derived`'s once it lands). This test pins the
    // current behaviour so a future "smart" change is intentional.
    Runtime::scope("set_same_value", || {
        let n = signal(7_i32);
        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || {
                let _ = n.get();
                runs.set(runs.get() + 1);
            });
        }
        assert_eq!(runs.get(), 1);
        n.set(7);
        n.set(7);
        n.set(7);
        assert_eq!(runs.get(), 4);
    });
}

// ----- batching -----

#[test]
fn nested_batch_drains_only_on_outer_exit() {
    Runtime::scope("nested_batch", || {
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
            batch(|| {
                n.set(2);
                n.set(3);
            });
            // Inner batch finished, but we are still in the outer
            // batch — no re-run yet.
            assert_eq!(runs.get(), 1);
            n.set(4);
        });
        // Outer batch exited — exactly one re-run.
        assert_eq!(runs.get(), 2);
        assert_eq!(n.peek(), 4);
    });
}

#[test]
fn batch_returns_inner_value() {
    Runtime::scope("batch_return", || {
        let result: i32 = batch(|| 42);
        assert_eq!(result, 42);

        let pair = batch(|| (1_u32, "two"));
        assert_eq!(pair, (1, "two"));
    });
}

#[test]
fn empty_batch_is_noop() {
    Runtime::scope("empty_batch", || {
        let n = signal(7_i32);
        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || {
                let _ = n.get();
                runs.set(runs.get() + 1);
            });
        }
        assert_eq!(runs.get(), 1);
        batch(|| {});
        assert_eq!(runs.get(), 1);
    });
}

// ----- update closure -----

#[test]
fn update_observes_current_value_and_writes_back() {
    Runtime::scope("update_observe", || {
        let n = signal(10_i32);
        let observed = Rc::new(Cell::new(-1));
        let observed_for = Rc::clone(&observed);
        n.update(move |v| {
            observed_for.set(*v);
            *v += 5;
        });
        assert_eq!(observed.get(), 10);
        assert_eq!(n.get(), 15);
    });
}

#[test]
fn update_can_call_back_into_other_signals() {
    // update() releases the runtime borrow before calling f, so f
    // is free to read or write OTHER signals. This is the bug fix
    // verified.
    Runtime::scope("update_callback", || {
        let n = signal(0_i32);
        let other = signal(100_i32);
        n.update(|v| {
            *v = other.peek();
            other.set(999);
        });
        assert_eq!(n.get(), 100);
        assert_eq!(other.get(), 999);
    });
}

#[test]
fn update_re_entry_on_same_signal_panics() {
    let result = std::panic::catch_unwind(|| {
        Runtime::scope("update_self_reentry", || {
            let n = signal(0_i32);
            n.update(|v| {
                *v = 1;
                // Calling update on n from within n.update should
                // panic with an actionable message.
                n.update(|w| *w = 2);
            });
        });
    });
    assert!(result.is_err(), "update re-entry on same signal must panic");
}

#[test]
fn get_on_self_during_own_update_panics() {
    let result = std::panic::catch_unwind(|| {
        Runtime::scope("get_during_update", || {
            let n = signal(0_i32);
            n.update(|v| {
                *v = 5;
                let _ = n.peek();
            });
        });
    });
    assert!(
        result.is_err(),
        "peek on self during own update closure must panic"
    );
}

// ----- owner tree -----

#[test]
fn nested_owner_re_entry_restores_outer_owner() {
    Runtime::scope("outer", || {
        let outer = Owner::current();
        Runtime::scope("inner", || {
            let inner = Owner::current();
            assert_ne!(outer, inner);
        });
        assert_eq!(Owner::current(), outer);
    });
}

#[test]
fn three_level_owner_disposal_runs_grandchildren_first() {
    Runtime::scope("three_levels", || {
        let log: Rc<std::cell::RefCell<Vec<&'static str>>> = Rc::default();

        let parent = Owner::new("parent", Owner::current());
        parent.with(|| {
            let child = Owner::new("child", Owner::current());
            child.with(|| {
                let grandchild = Owner::new("grandchild", Owner::current());
                grandchild.with(|| {
                    let log_g = Rc::clone(&log);
                    on_cleanup(move || log_g.borrow_mut().push("grandchild"));
                });
                let log_c = Rc::clone(&log);
                on_cleanup(move || log_c.borrow_mut().push("child"));
            });
            let log_p = Rc::clone(&log);
            on_cleanup(move || log_p.borrow_mut().push("parent"));
        });
        parent.dispose();

        assert_eq!(*log.borrow(), vec!["grandchild", "child", "parent"]);
    });
}

#[test]
fn double_dispose_is_noop() {
    Runtime::scope("double_dispose", || {
        let owner = Owner::new("doomed", Owner::current());
        owner.dispose();
        owner.dispose(); // must not panic
    });
}

#[test]
fn cleanup_can_read_signals_in_outer_scope() {
    // Cleanup runs after the disposing owner's children are gone
    // but before its own signals are invalidated. Signals belonging
    // to outer scopes must remain readable.
    Runtime::scope("cleanup_reads_sibling", || {
        let counter = signal(0_i32);
        let cleanup_saw = Rc::new(Cell::new(-1));
        let cleanup_for = Rc::clone(&cleanup_saw);

        let child = Owner::new("child", Owner::current());
        child.with(|| {
            on_cleanup(move || cleanup_for.set(counter.peek()));
        });
        counter.set(42);
        child.dispose();

        assert_eq!(cleanup_saw.get(), 42);
    });
}

#[test]
fn multiple_cleanups_in_one_owner_run_lifo() {
    Runtime::scope("multi_cleanup_lifo", || {
        let log: Rc<std::cell::RefCell<Vec<u32>>> = Rc::default();
        let owner = Owner::new("owner", Owner::current());
        owner.with(|| {
            for i in 0..5_u32 {
                let log = Rc::clone(&log);
                on_cleanup(move || log.borrow_mut().push(i));
            }
        });
        owner.dispose();
        assert_eq!(*log.borrow(), vec![4, 3, 2, 1, 0]);
    });
}

// ----- scheduling chains -----

#[test]
fn effect_writing_to_unrelated_signal_chains_to_other_effects() {
    Runtime::scope("indirect_chain", || {
        let trigger = signal(0_i32);
        let target = signal(0_i32);
        let saw = Rc::new(Cell::new(-1));

        // Effect A: reads trigger, writes target.
        effect(move || target.set(trigger.get() * 2));

        // Effect B: reads target.
        let saw_for = Rc::clone(&saw);
        effect(move || saw_for.set(target.get()));

        assert_eq!(saw.get(), 0);
        trigger.set(5);
        assert_eq!(saw.get(), 10);
        trigger.set(-3);
        assert_eq!(saw.get(), -6);
    });
}

#[cfg(debug_assertions)]
#[test]
fn mutual_recursion_through_separate_effects_panics() {
    let result = std::panic::catch_unwind(|| {
        Runtime::scope("mutual_recursion", || {
            let a = signal(0_i32);
            let b = signal(0_i32);

            // Effect 1: a -> writes b.
            effect(move || b.set(a.get() + 1));
            // Effect 2: b -> writes a. This re-fires effect 1
            // through a's subscription, and effect 1 is currently
            // running — cycle.
            effect(move || a.set(b.get() + 1));
        });
    });
    assert!(result.is_err(), "mutual recursion must panic");
}

#[cfg(not(debug_assertions))]
#[test]
fn mutual_recursion_through_separate_effects_warns_and_skips_in_release() {
    Runtime::scope("mutual_recursion", || {
        let a = signal(0_i32);
        let b = signal(0_i32);

        effect(move || b.set(a.get() + 1));
        effect(move || a.set(b.get() + 1));

        assert_eq!(a.peek(), 2);
        assert_eq!(b.peek(), 3);
    });
}

// ----- handle identity -----

#[test]
fn signal_handles_are_distinct_per_construction() {
    Runtime::scope("distinct_signals", || {
        let a = signal(1_i32);
        let b = signal(1_i32);
        // Both hold the same value but the handles are not equal
        // (they index different arena slots).
        a.set(99);
        assert_eq!(a.get(), 99);
        assert_eq!(b.get(), 1, "b should be unaffected by a's writes");
    });
}

#[test]
fn owner_handles_with_same_id_compare_equal() {
    Runtime::scope("owner_id_eq", || {
        let id_a = Owner::current().expect("scope establishes current owner");
        let id_b = Owner::current().expect("still in scope");
        assert_eq!(id_a, id_b);
    });
}
