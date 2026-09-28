use super::*;

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

        b.set(200);
        assert_eq!(last.get(), 1);

        a.set(2);
        assert_eq!(last.get(), 2);

        cond.set(false);
        assert_eq!(last.get(), 200);

        a.set(99);
        assert_eq!(last.get(), 200);

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
fn set_with_same_value_does_not_fire_effects() {
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
        assert_eq!(runs.get(), 1, "equal sets are no-ops");
        n.set_always(7);
        assert_eq!(runs.get(), 2, "set_always always fires");
    });
}
