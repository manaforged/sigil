//! Integration tests for `sigil-reactive`.
//!
//! These are behaviour tests, not unit tests. Each test exercises
//! one of the contracts documented in DR-005 end-to-end: effect
//! subscription, disposal ordering, batched coalescing, cycle
//! detection, and stale-handle behaviour.
//!
//! Tests here run serially by default because `sigil-reactive`'s
//! runtime is thread-local. Each test gets a fresh `Runtime::scope`.

#![allow(missing_docs)]

// `criterion` is a dev-dep (for benches). `smallvec` is a regular
// dep (subscriber storage). Neither is used in this test file; the
// aliases satisfy `unused_crate_dependencies` which flows into every
// integration-test binary.
use actions as _;
use criterion as _;
use smallvec as _;
use tokio as _;

use std::cell::Cell;
use std::rc::Rc;

use reactive::{Owner, Runtime, batch, effect, on_cleanup, signal};

#[test]
fn signal_get_and_set_roundtrip() {
    Runtime::scope("signal_get_and_set_roundtrip", || {
        let n = signal(0_i32);
        assert_eq!(n.get(), 0);
        n.set(7);
        assert_eq!(n.get(), 7);
    });
}

#[test]
fn effect_runs_once_on_register_and_again_on_write() {
    Runtime::scope("effect_fires_on_write", || {
        let n = signal(0_i32);
        let seen = Rc::new(Cell::new(-1));
        {
            let seen = Rc::clone(&seen);
            effect(move || {
                seen.set(n.get());
            });
        }
        assert_eq!(seen.get(), 0, "effect ran once on registration");
        n.set(42);
        assert_eq!(seen.get(), 42, "effect re-ran after write");
    });
}

#[test]
fn peek_does_not_subscribe() {
    Runtime::scope("peek_does_not_subscribe", || {
        let n = signal(1_i32);
        let fires = Rc::new(Cell::new(0_u32));
        {
            let fires = Rc::clone(&fires);
            effect(move || {
                let _ = n.peek();
                fires.set(fires.get() + 1);
            });
        }
        assert_eq!(fires.get(), 1);
        n.set(2);
        assert_eq!(fires.get(), 1, "peek should not subscribe");
    });
}

#[test]
fn batch_coalesces_dependent_effect_to_one_run() {
    Runtime::scope("batch_coalesces", || {
        let a = signal(0_i32);
        let b = signal(0_i32);
        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || {
                let _ = a.get();
                let _ = b.get();
                runs.set(runs.get() + 1);
            });
        }
        assert_eq!(runs.get(), 1, "initial registration run");
        batch(|| {
            a.set(1);
            a.set(2);
            b.set(3);
        });
        assert_eq!(
            runs.get(),
            2,
            "three writes in a batch collapse to one re-run"
        );
    });
}

#[test]
fn on_cleanup_runs_in_lifo_order_on_owner_disposal() {
    let log: Rc<std::cell::RefCell<Vec<&'static str>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));
    let log_for_scope = Rc::clone(&log);
    Runtime::scope("lifo_cleanup_outer", move || {
        let parent = Owner::new("parent", Owner::current());
        parent.with(|| {
            let log_a = Rc::clone(&log_for_scope);
            on_cleanup(move || log_a.borrow_mut().push("a"));
            let log_b = Rc::clone(&log_for_scope);
            on_cleanup(move || log_b.borrow_mut().push("b"));
        });
        parent.dispose();
    });
    assert_eq!(*log.borrow(), vec!["b", "a"], "LIFO ordering of cleanups");
}

#[test]
fn child_owners_dispose_before_parent_cleanups_run() {
    Runtime::scope("child_then_parent_cleanups", || {
        let log: Rc<std::cell::RefCell<Vec<&'static str>>> = Rc::default();

        let parent = Owner::new("parent", Owner::current());
        parent.with(|| {
            let log_child = Rc::clone(&log);
            let child = Owner::new("child", Owner::current());
            child.with(|| {
                let log_cleanup = Rc::clone(&log_child);
                on_cleanup(move || log_cleanup.borrow_mut().push("child"));
            });
            let log_parent = Rc::clone(&log);
            on_cleanup(move || log_parent.borrow_mut().push("parent"));
            // Note: we don't dispose `child` explicitly — parent
            // disposal cascades to children.
            let _ = child;
        });
        parent.dispose();

        assert_eq!(
            *log.borrow(),
            vec!["child", "parent"],
            "child cleanup fires before parent cleanup"
        );
    });
}

#[test]
fn signal_read_after_owner_dispose_panics() {
    let result = std::panic::catch_unwind(|| {
        let captured = Runtime::scope("stale_signal_outer", || {
            let parent = Owner::new("parent", Owner::current());
            let n = parent.with(|| signal(99_i32));
            parent.dispose();
            n
        });
        // Reading a signal whose owner is disposed must panic, not
        // return stale data.
        let _ = captured.peek();
    });
    assert!(result.is_err(), "reading disposed signal should panic");
}

#[cfg(debug_assertions)]
#[test]
fn cycle_detection_panics_on_self_reentry() {
    let result = std::panic::catch_unwind(|| {
        Runtime::scope("cycle", || {
            let n = signal(0_i32);
            effect(move || {
                let current = n.get();
                if current < 3 {
                    // Writing from inside the effect would re-enter
                    // it before it finishes — that's the cycle.
                    n.set(current + 1);
                }
            });
        });
    });
    assert!(result.is_err(), "self-re-entry must panic");
}

#[cfg(not(debug_assertions))]
#[test]
fn cycle_detection_warns_and_skips_self_reentry_in_release() {
    Runtime::scope("cycle", || {
        let n = signal(0_i32);
        effect(move || {
            let current = n.get();
            if current < 3 {
                n.set(current + 1);
            }
        });
        assert_eq!(n.peek(), 1, "release skips the re-entrant run");
    });
}

#[test]
fn disposed_effect_stops_re_running() {
    let runs = Rc::new(Cell::new(0_u32));
    {
        let runs = Rc::clone(&runs);
        Runtime::scope("dispose_effect", move || {
            let n = signal(0_i32);
            let child = Owner::new("effect_scope", Owner::current());
            child.with(|| {
                let runs = Rc::clone(&runs);
                effect(move || {
                    let _ = n.get();
                    runs.set(runs.get() + 1);
                });
            });
            assert_eq!(runs.get(), 1);
            child.dispose();
            n.set(1); // should not re-run the disposed effect
            assert_eq!(runs.get(), 1);
        });
    }
}
