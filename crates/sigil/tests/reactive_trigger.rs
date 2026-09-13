#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::reactive::{EventChannel, Runtime};

#[test]
fn event_channel_multiple_subscribers_all_fire() {
    Runtime::scope("event_channel_multiple_subscribers_all_fire", || {
        let channel = EventChannel::new();
        let first = Rc::new(Cell::new(0_i32));
        let second = Rc::new(Cell::new(0_i32));

        let first_for_callback = Rc::clone(&first);
        let _first_subscription = channel.subscribe(move |_value| {
            first_for_callback.set(first_for_callback.get() + 1);
        });
        let second_for_callback = Rc::clone(&second);
        let _second_subscription = channel.subscribe(move |_value| {
            second_for_callback.set(second_for_callback.get() + 1);
        });

        channel.emit(1);

        assert_eq!(first.get(), 1);
        assert_eq!(second.get(), 1);
    });
}

#[test]
fn event_channel_drop_removes_subscriber() {
    Runtime::scope("event_channel_drop_removes_subscriber", || {
        let channel = EventChannel::new();
        let count = Rc::new(Cell::new(0_i32));

        let count_for_callback = Rc::clone(&count);
        let subscription = channel.subscribe(move |_value| {
            count_for_callback.set(count_for_callback.get() + 1);
        });
        drop(subscription);

        channel.emit(1);

        assert_eq!(count.get(), 0);
    });
}

#[test]
fn event_channel_works_without_runtime_scope() {
    let channel = EventChannel::new();
    let count = Rc::new(Cell::new(0_i32));
    let count_for_callback = Rc::clone(&count);
    let _subscription = channel.subscribe(move |_value| {
        count_for_callback.set(count_for_callback.get() + 1);
    });
    channel.emit(1);
    assert_eq!(count.get(), 1);
}
