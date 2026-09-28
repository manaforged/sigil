use super::*;

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
        owner.dispose();
    });
}

#[test]
fn cleanup_can_read_signals_in_outer_scope() {
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

#[test]
fn effect_writing_to_unrelated_signal_chains_to_other_effects() {
    Runtime::scope("indirect_chain", || {
        let trigger = signal(0_i32);
        let target = signal(0_i32);
        let saw = Rc::new(Cell::new(-1));

        effect(move || target.set(trigger.get() * 2));

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

            effect(move || b.set(a.get() + 1));
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

#[test]
fn signal_handles_are_distinct_per_construction() {
    Runtime::scope("distinct_signals", || {
        let a = signal(1_i32);
        let b = signal(1_i32);
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
