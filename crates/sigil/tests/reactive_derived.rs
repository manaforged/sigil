#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::reactive::{Owner, Runtime, derived, derived_eq, effect, signal};

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
    Runtime::scope("derived_eq_memoizes", || {
        let n = signal(3_i32);
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

        n.set(7);
        assert_eq!(
            runs.get(),
            1,
            "memoized derived did not notify on equal-value update"
        );
        n.set(12);
        assert_eq!(runs.get(), 1);

        n.set(-1);
        assert_eq!(runs.get(), 2, "memoized derived notified on value change");

        n.set(-5);
        assert_eq!(runs.get(), 2);
    });
}

#[test]
fn derived_of_derived_propagates_correctly() {
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
    Runtime::scope("derived_eq_silent_on_unchanged", || {
        let n = signal(10_i32);
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

        for v in [0, 1, 25, 49, 50] {
            n.set(v);
        }
        assert_eq!(
            runs.get(),
            1,
            "no outer notifications for same-bucket writes"
        );

        n.set(100);
        assert_eq!(runs.get(), 2);

        n.set(75);
        assert_eq!(runs.get(), 2);
    });
}

#[test]
fn derived_calls_f_exactly_once_per_update() {
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

        n.set(2);
        assert_eq!(calls.get(), 2);
        n.set(3);
        assert_eq!(calls.get(), 3);
    });
}

#[test]
fn derived_is_disposed_with_owner() {
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
    Runtime::scope("derived_survives_reruns", || {
        let n = signal(0_i32);
        let d = derived(move || n.get() + 100);

        assert_eq!(d.get(), 100);

        for v in 1..50 {
            n.set(v);
            assert_eq!(d.get(), v + 100);
        }
    });
}

#[test]
fn derived_works_with_non_partial_eq_type() {
    #[derive(Clone)]
    struct Opaque(#[expect(dead_code)] Rc<dyn Fn(i32) -> i32>);

    Runtime::scope("derived_non_partial_eq", || {
        let which = signal(0_i32);
        let op: sigil::reactive::Signal<Opaque> = derived(move || {
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
