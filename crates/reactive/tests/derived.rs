//! Integration tests for `derived` / `derived_eq`.
//!
//! These pin the contract laid out in DR-005 and the derived module
//! doc comments: `f` is called exactly once per dependency update
//! (including registration); the returned signal tracks like any
//! other; `derived_eq` memoizes by `PartialEq` so equal re-runs do
//! not notify; the backing signal is owned by the enclosing scope
//! and disposes with it.

#![allow(missing_docs)]

// Silence `unused_crate_dependencies` — criterion is a dev-dep for
// benches, smallvec a regular dep for subscriber storage; neither is
// referenced in this test file.
use criterion as _;
use sigil as _;
use sigil_actions as _;
use smallvec as _;
use tokio as _;

use std::cell::Cell;
use std::rc::Rc;

use sigil_reactive::{Owner, Runtime, derived, derived_eq, effect, signal};

#[test]
fn derived_computes_initial_value_on_registration() {
    Runtime::scope("derived_initial", || {
        let n = signal(3_i32);
        let doubled = derived(move || n.get() * 2);
        assert_eq!(
            doubled.get(),
            6,
            "initial derived value available after registration"
        );
    });
}

#[test]
fn derived_re_runs_on_subscribed_signal_change() {
    Runtime::scope("derived_re_runs", || {
        let n = signal(5_i32);
        let squared = derived(move || n.get() * n.get());
        assert_eq!(squared.get(), 25);
        n.set(7);
        assert_eq!(squared.get(), 49, "derived recomputes on dependency write");
        n.set(0);
        assert_eq!(squared.get(), 0);
    });
}

#[test]
fn derived_reads_are_tracked_by_outer_effects() {
    Runtime::scope("derived_tracks_outer", || {
        let n = signal(1_i32);
        let doubled = derived(move || n.get() * 2);

        let seen = Rc::new(Cell::new(-1_i32));
        {
            let seen = Rc::clone(&seen);
            effect(move || {
                seen.set(doubled.get());
            });
        }
        assert_eq!(seen.get(), 2, "outer effect saw initial derived value");

        n.set(10);
        assert_eq!(seen.get(), 20, "outer effect re-ran when derived updated");
    });
}

#[test]
fn derived_eq_memoizes_equal_values() {
    // An input change that produces the same derived value must not
    // fire outer effects subscribed to the derived.
    Runtime::scope("derived_eq_memoizes", || {
        let n = signal(3_i32);
        // `is_positive` only flips when the sign flips, not on every
        // input change.
        let is_pos = derived_eq(move || n.get() > 0);

        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || {
                let _ = is_pos.get();
                runs.set(runs.get() + 1);
            });
        }
        assert_eq!(runs.get(), 1, "registration run only");

        // Changing n within the same sign leaves is_pos unchanged.
        n.set(7);
        assert_eq!(
            runs.get(),
            1,
            "memoized derived did not notify on equal-value update"
        );
        n.set(12);
        assert_eq!(runs.get(), 1);

        // Flipping the sign does notify.
        n.set(-1);
        assert_eq!(runs.get(), 2, "memoized derived notified on value change");

        // Staying negative does not.
        n.set(-5);
        assert_eq!(runs.get(), 2);
    });
}

#[test]
fn derived_of_derived_propagates_correctly() {
    // A chain derived -> derived -> effect. Writes to the root
    // signal must flow through both layers.
    Runtime::scope("derived_chain", || {
        let n = signal(2_i32);
        let plus_one = derived(move || n.get() + 1);
        let plus_ten = derived(move || plus_one.get() + 10);

        let seen = Rc::new(Cell::new(-1_i32));
        {
            let seen = Rc::clone(&seen);
            effect(move || seen.set(plus_ten.get()));
        }
        assert_eq!(seen.get(), 13, "chain: (2 + 1) + 10 = 13");

        n.set(100);
        assert_eq!(seen.get(), 111, "chain propagated: (100 + 1) + 10 = 111");

        n.set(-5);
        assert_eq!(seen.get(), 6, "(-5 + 1) + 10 = 6");
    });
}

#[test]
fn derived_eq_does_not_fire_outer_effects_when_value_unchanged() {
    // Pin the memoization contract directly: writes that change the
    // dependency but not the derived value must be fully silent.
    Runtime::scope("derived_eq_silent_on_unchanged", || {
        let n = signal(10_i32);
        // Bucket the input into "0..=50" vs "51+" — many distinct
        // inputs map to the same bucket.
        let bucket = derived_eq(move || n.get() / 51);

        let runs = Rc::new(Cell::new(0_u32));
        {
            let runs = Rc::clone(&runs);
            effect(move || {
                let _ = bucket.get();
                runs.set(runs.get() + 1);
            });
        }
        assert_eq!(runs.get(), 1);

        // All of these stay in bucket 0.
        for v in [0, 1, 25, 49, 50] {
            n.set(v);
        }
        assert_eq!(
            runs.get(),
            1,
            "no outer notifications for same-bucket writes"
        );

        // Cross the bucket boundary.
        n.set(100);
        assert_eq!(runs.get(), 2);

        // Back within bucket 1.
        n.set(75);
        assert_eq!(runs.get(), 2);
    });
}

#[test]
fn derived_calls_f_exactly_once_per_update() {
    // Pin the no-2x-registration contract: a call counter inside `f`
    // increments by exactly 1 per external dependency write, and is
    // already at 1 immediately after `derived(...)` returns.
    Runtime::scope("derived_f_once_per_update", || {
        let n = signal(0_i32);
        let calls = Rc::new(Cell::new(0_u32));

        let calls_for_f = Rc::clone(&calls);
        let d = derived(move || {
            calls_for_f.set(calls_for_f.get() + 1);
            n.get() * 2
        });
        assert_eq!(calls.get(), 1, "f called exactly once on registration");
        assert_eq!(d.get(), 0);

        n.set(5);
        assert_eq!(calls.get(), 2, "one write -> one f call");
        assert_eq!(d.get(), 10);

        n.set(6);
        assert_eq!(calls.get(), 3, "second write -> third f call");

        n.set(7);
        assert_eq!(calls.get(), 4);
    });
}

#[test]
fn derived_eq_calls_f_exactly_once_per_update_even_on_equal_values() {
    // Memoization skips the outer notification, but `f` itself still
    // runs once per input change (that is how we know the values are
    // equal in the first place).
    Runtime::scope("derived_eq_f_once_per_update", || {
        let n = signal(1_i32);
        let calls = Rc::new(Cell::new(0_u32));

        let calls_for_f = Rc::clone(&calls);
        let is_pos = derived_eq(move || {
            calls_for_f.set(calls_for_f.get() + 1);
            n.get() > 0
        });
        assert_eq!(calls.get(), 1);
        assert!(is_pos.get());

        // Same-sign writes — `f` still runs per write (checks the
        // result), but memoization suppresses outer fan-out.
        n.set(2);
        assert_eq!(calls.get(), 2);
        n.set(3);
        assert_eq!(calls.get(), 3);
    });
}

#[test]
fn derived_is_disposed_with_owner() {
    // Capture a derived signal from a child owner, dispose the child,
    // then assert that reading the derived panics with a
    // stale-handle message.
    let captured = Runtime::scope("derived_dispose_outer", || {
        let child = Owner::new("derived_child", Owner::current());
        let d = child.with(|| {
            let n = signal(0_i32);
            derived(move || n.get() + 1)
        });
        child.dispose();
        d
    });
    let result = std::panic::catch_unwind(|| {
        let _ = captured.peek();
    });
    assert!(
        result.is_err(),
        "reading a derived whose owner was disposed must panic"
    );
}

#[test]
fn derived_survives_across_multiple_effect_runs() {
    // Guards against a regression where the backing signal is
    // parented to the tracking effect's per-run owner (which is
    // disposed on each re-run) instead of the enclosing scope.
    Runtime::scope("derived_survives_reruns", || {
        let n = signal(0_i32);
        let d = derived(move || n.get() + 100);

        assert_eq!(d.get(), 100);

        // Drive many updates — if the backing signal were parented
        // to the effect's per-run owner, the second write would find
        // a disposed signal and panic.
        for v in 1..50 {
            n.set(v);
            assert_eq!(d.get(), v + 100);
        }
    });
}

#[test]
fn derived_works_with_non_partial_eq_type() {
    // `derived` requires only `Clone + 'static`. Using a type that is
    // not `PartialEq` (here, a function pointer wrapped in a newtype
    // around an `Rc<dyn Fn>`) proves the minimal bound is enough.
    #[derive(Clone)]
    struct Opaque(#[expect(dead_code)] Rc<dyn Fn(i32) -> i32>);

    Runtime::scope("derived_non_partial_eq", || {
        let which = signal(0_i32);
        let op: sigil_reactive::Signal<Opaque> = derived(move || {
            if which.get() == 0 {
                Opaque(Rc::new(|x| x + 1))
            } else {
                Opaque(Rc::new(|x| x * 2))
            }
        });
        let _first = op.get();
        which.set(1);
        let _second = op.get();
    });
}
