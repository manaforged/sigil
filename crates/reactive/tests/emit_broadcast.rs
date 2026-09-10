#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil_reactive::{Entity, Runtime};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Pong(u32);

#[test]
fn emit_broadcasts_full_payload_to_every_subscriber() {
    Runtime::scope("emit_broadcast", || {
        let producer = Entity::<()>::new("producer", |_cx| ());

        let log_a: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));
        let log_b: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));
        let log_c: Rc<RefCell<Vec<Pong>>> = Rc::new(RefCell::new(Vec::new()));

        let install = |log: Rc<RefCell<Vec<Pong>>>| {
            Entity::<()>::new("observer", move |cx| {
                let sub = cx.subscribe::<(), Pong>(&producer, move |_self, _e, ev, _cx| {
                    log.borrow_mut().push(ev.clone());
                });
                core::mem::forget(sub);
            })
        };

        let _obs_a = install(Rc::clone(&log_a));
        let _obs_b = install(Rc::clone(&log_b));
        let _obs_c = install(Rc::clone(&log_c));

        producer.update(|(), cx| {
            cx.emit(Pong(7));
        });

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
        let producer = Entity::<()>::new("producer", |_cx| ());

        producer.update(|(), cx| {
            cx.emit(Pong(0));
            cx.emit(Pong(1));
            cx.emit(Pong(2));
        });
    });
}

#[test]
fn emit_single_subscriber_receives_payload() {
    Runtime::scope("emit_single", || {
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
