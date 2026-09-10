#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::cell::Cell;
use std::rc::Rc;

use sigil_reactive::{Entity, Runtime, global, observe_global, set_global};

#[test]
fn entity_new_returns_handle_with_owned_value() {
    Runtime::scope("entity_new", || {
        let counter = Entity::<i32>::new("counter", |_cx| 42);
        let value = counter.read(|v| *v);
        assert_eq!(value, 42);
    });
}

#[test]
fn entity_update_mutates_in_place_and_read_observes_it() {
    Runtime::scope("entity_update", || {
        let counter = Entity::<i32>::new("counter", |_cx| 0);
        counter.update(|v, _cx| *v += 7);
        counter.update(|v, _cx| *v += 3);
        let after = counter.read(|v| *v);
        assert_eq!(after, 10);
    });
}

#[test]
fn observe_global_callback_fires_after_set_global_turn() {
    Runtime::scope("observe_global_fires", || {
        #[derive(Default)]
        struct CounterCfg(#[allow(dead_code)] i32);

        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        let _sub = observe_global::<CounterCfg>(move || {
            fired_clone.set(fired_clone.get() + 1);
        });

        assert_eq!(fired.get(), 0);

        set_global(CounterCfg(5));
        assert_eq!(fired.get(), 1, "observer should fire on set_global");

        set_global(CounterCfg(9));
        assert_eq!(fired.get(), 2, "observer should fire on each set");
    });
}

#[test]
fn subscription_drop_unregisters_observer() {
    Runtime::scope("sub_raii", || {
        #[derive(Default)]
        struct Flagged;

        let fired = Rc::new(Cell::new(0_u32));
        let fired_clone = Rc::clone(&fired);

        let sub = observe_global::<Flagged>(move || {
            fired_clone.set(fired_clone.get() + 1);
        });

        set_global(Flagged);
        assert_eq!(fired.get(), 1);

        drop(sub);

        set_global(Flagged);
        assert_eq!(
            fired.get(),
            1,
            "dropped Subscription must not re-fire observer"
        );
    });
}

#[test]
fn set_global_round_trips_through_global_reader() {
    Runtime::scope("global_round_trip", || {
        #[derive(Default)]
        struct Config {
            scale: f32,
        }

        set_global(Config { scale: 2.5 });
        let read_back = global::<Config, f32>(|c| c.scale);
        assert!((read_back - 2.5).abs() < f32::EPSILON);

        set_global(Config { scale: 7.0 });
        let read_back2 = global::<Config, f32>(|c| c.scale);
        assert!((read_back2 - 7.0).abs() < f32::EPSILON);
    });
}

#[test]
fn entity_value_persists_across_multiple_reads_and_updates() {
    Runtime::scope("entity_persistence", || {
        let log = Entity::<Vec<i32>>::new("log", |_cx| Vec::new());

        log.update(|v, _cx| v.push(1));
        log.update(|v, _cx| v.push(2));
        log.update(|v, _cx| v.push(3));
        let snapshot = log.read(Clone::clone);
        assert_eq!(snapshot, vec![1, 2, 3]);

        log.update(|v, _cx| v.push(4));
        let after = log.read(Vec::len);
        assert_eq!(after, 4);
    });
}
