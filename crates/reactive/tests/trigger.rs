//! Integration tests for typed state and edge triggers.

#![allow(missing_docs)]

use criterion as _;
use sigil as _;
use sigil_actions as _;
use smallvec as _;
use tokio as _;

use std::cell::Cell;
use std::rc::Rc;

use sigil_reactive::{EventChannel, Runtime, Trigger, signal};

#[test]
fn trigger_state_subscribe_fires_on_signal_change() {
    Runtime::scope("trigger_state_subscribe_fires_on_signal_change", || {
        let open = signal(false);
        let trigger = Trigger::State(open);
        let seen = Rc::new(Cell::new(false));

        let seen_for_callback = Rc::clone(&seen);
        let _subscription = trigger.subscribe(move |value| {
            seen_for_callback.set(value);
        });

        open.set(true);

        assert!(seen.get());
    });
}

#[test]
fn trigger_edge_emit_fires_subscribers() {
    Runtime::scope("trigger_edge_emit_fires_subscribers", || {
        let channel = EventChannel::new();
        let trigger = Trigger::Edge(channel);
        let seen = Rc::new(Cell::new(0_u32));

        let seen_for_callback = Rc::clone(&seen);
        let _subscription = trigger.subscribe(move |value| {
            seen_for_callback.set(value);
        });

        channel.emit(42);

        assert_eq!(seen.get(), 42);
    });
}

#[test]
#[should_panic(expected = "Trigger::current called on an edge trigger with no state")]
fn trigger_edge_no_current_state() {
    Runtime::scope("trigger_edge_no_current_state", || {
        let trigger = Trigger::Edge(EventChannel::<bool>::new());

        let _ = trigger.current();
    });
}

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
