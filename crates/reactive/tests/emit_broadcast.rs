#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! W4.99.B4 — `Context::emit` broadcasts the full payload to every
//! subscriber.
//!
//! Pre-fix, the implementation handed the boxed payload to the FIRST
//! subscriber and `event: None` to every subsequent one — a silent
//! semantic divergence from GPUI / Dioxus / Bevy. These tests pin the
//! corrected broadcast-all contract.

#![allow(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil_reactive::{Entity, Runtime};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Pong(u32);

#[test]
fn emit_broadcasts_full_payload_to_every_subscriber() {
    Runtime::scope("emit_broadcast", || {
        // Three observers all subscribed to the same event type on the
        // same producer entity. Each captures the received payloads
        // into its own log.
        let producer = Entity::<()>::new("producer", |_cx| ());

        let log_a: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));
        let log_b: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));
        let log_c: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));

        let install = |log: Rc<RefCell<Vec<Pong>>>| {
            Entity::<()>::new("observer", move |cx| {
                let sub = cx.subscribe::<(), Pong>(&producer, move |_self, _e, ev, _cx| {
                    log.borrow_mut().push(ev.clone());
                });
                // Keep the subscription alive for the duration of the
                // Runtime::scope: leak it intentionally. Subscribers
                // also unsubscribe via the owner-cascade safety net
                // when the owner disposes at end-of-scope.
                core::mem::forget(sub);
            })
        };

        let _obs_a = install(Rc::clone(&log_a));
        let _obs_b = install(Rc::clone(&log_b));
        let _obs_c = install(Rc::clone(&log_c));

        // Emit a single event from inside the producer's `update`.
        producer.update(|(), cx| {
            cx.emit(Pong(7));
        });

        // All three logs must contain the same payload — full broadcast.
        assert_eq!(
            log_a.borrow().as_slice(),
            &[Pong(7)],
            "subscriber A must receive the cloned payload"
        );
        assert_eq!(
            log_b.borrow().as_slice(),
            &[Pong(7)],
            "subscriber B must receive the cloned payload (regression: \
             pre-fix this slot was None)"
        );
        assert_eq!(
            log_c.borrow().as_slice(),
            &[Pong(7)],
            "subscriber C must receive the cloned payload (regression: \
             pre-fix this slot was None)"
        );
    });
}

#[test]
fn emit_with_zero_subscribers_is_noop() {
    Runtime::scope("emit_empty", || {
        // No subscribers — emit must not panic, not allocate a
        // payload box (verified indirectly: the early-return branch),
        // and must not enqueue any pending observers.
        let producer = Entity::<()>::new("producer", |_cx| ());

        producer.update(|(), cx| {
            cx.emit(Pong(0));
            cx.emit(Pong(1));
            cx.emit(Pong(2));
        });

        // Survives end-of-scope drain without panic — that's the
        // contract.
    });
}

#[test]
fn emit_single_subscriber_receives_payload() {
    Runtime::scope("emit_single", || {
        // The single-subscriber case must still deliver the payload —
        // the optimisation that "the last subscriber gets the original
        // ev" must not regress to None for N=1.
        let producer = Entity::<()>::new("producer", |_cx| ());

        let log: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_observer = Rc::clone(&log);
        let _obs = Entity::<()>::new("observer", move |cx| {
            let sub = cx.subscribe::<(), Pong>(&producer, move |_self, _e, ev, _cx| {
                log_for_observer.borrow_mut().push(ev.clone());
            });
            core::mem::forget(sub);
        });

        producer.update(|(), cx| cx.emit(Pong(42)));

        assert_eq!(log.borrow().as_slice(), &[Pong(42)]);
    });
}
