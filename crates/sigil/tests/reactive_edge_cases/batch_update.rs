use super::*;

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
            assert_eq!(runs.get(), 1);
            n.set(4);
        });
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
