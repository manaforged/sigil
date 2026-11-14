#![allow(clippy::panic, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::reactive::Owner;
use crate::tasks::AsyncRuntime;
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::tests::spin_until;

fn setup() -> (AsyncRuntime, Owner) {
    let rt = AsyncRuntime::new().expect("build tokio runtime");
    let handle = Owner::current().expect("inside a reactive scope");
    (rt, handle)
}

#[test]
fn drain_preserves_enqueue_order() {
    Owner::scope("drain_preserves_enqueue_order", || {
        let (mut scheduler, handle) = setup();
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let done = Arc::new(AtomicBool::new(false));
        let dispatcher = scheduler.spawn_parts().dispatcher;
        {
            let (order, done) = (Arc::clone(&order), Arc::clone(&done));
            scheduler.spawn(handle, async move {
                for label in ["a", "b", "c"] {
                    let order = Arc::clone(&order);
                    dispatch_to_owner(&dispatcher, handle, move || {
                        order.lock().expect("mutex").push(label);
                    });
                    tokio::task::yield_now().await;
                }
                done.store(true, Ordering::SeqCst);
            });
        }
        assert!(
            spin_until(Duration::from_secs(5), || done.load(Ordering::SeqCst)),
            "the task should enqueue a, b, and c"
        );
        assert_eq!(scheduler.pump(), 3, "three callbacks should have drained");
        assert_eq!(
            *order.lock().expect("mutex"),
            vec!["a", "b", "c"],
            "drain preserves enqueue order"
        );
    });
}

#[test]
fn dispatch_to_owner_sets_current_owner_during_callback() {
    Owner::scope(
        "dispatch_to_owner_sets_current_owner_during_callback",
        || {
            let (mut scheduler, handle) = setup();
            let expected = handle;
            let observed: Arc<std::sync::Mutex<Option<crate::reactive::Owner>>> =
                Arc::new(std::sync::Mutex::new(None));
            let observed_cb = Arc::clone(&observed);

            let dispatcher = scheduler.spawn_parts().dispatcher;
            dispatch_to_owner(&dispatcher, handle, move || {
                *observed_cb.lock().expect("mutex") = Owner::current();
            });

            scheduler.pump();

            let observed_id = *observed.lock().expect("mutex");
            assert_eq!(
                observed_id,
                Some(expected),
                "dispatch_to_owner closure sees the dispatched owner as current"
            );
        },
    );
}

#[test]
fn drain_does_not_process_callbacks_enqueued_during_drain() {
    Owner::scope(
        "drain_does_not_process_callbacks_enqueued_during_drain",
        || {
            let (mut scheduler, handle) = setup();
            let a_ran = Arc::new(AtomicBool::new(false));
            let b_ran = Arc::new(AtomicBool::new(false));

            let dispatcher_for_a = scheduler.spawn_parts().dispatcher;
            let a_ran_cb = Arc::clone(&a_ran);
            let b_ran_for_b = Arc::clone(&b_ran);
            scheduler
                .enqueue_main(
                    handle,
                    Box::new(move || {
                        a_ran_cb.store(true, Ordering::SeqCst);
                        dispatch_to_owner(&dispatcher_for_a, handle, move || {
                            b_ran_for_b.store(true, Ordering::SeqCst);
                        });
                    }),
                )
                .expect("enqueue A");

            let ran_first = scheduler.pump();
            assert_eq!(ran_first, 1, "first drain runs exactly A (budget == 1)");
            assert!(a_ran.load(Ordering::SeqCst), "A ran");
            assert!(
                !b_ran.load(Ordering::SeqCst),
                "B was enqueued during drain and must defer to the next cycle"
            );

            let ran_second = scheduler.pump();
            assert_eq!(ran_second, 1, "second drain runs exactly B");
            assert!(
                b_ran.load(Ordering::SeqCst),
                "B runs on the next drain cycle"
            );
        },
    );
}

#[test]
fn a_writer_made_in_a_scope_that_ended_still_writes() {
    Owner::scope("writer-outlives-scope", || {
        let mut tasks = AsyncRuntime::new().expect("build tokio runtime");
        tasks.install_current();
        let total = crate::reactive::signal(0);
        let writer = Owner::scope("child", || total.writer());
        writer.set(7).expect("send");
        let _ = tasks.pump();
        assert_eq!(total.peek(), 7);
    });
}

#[test]
fn a_write_to_a_disposed_signal_is_dropped_without_a_panic() {
    Owner::scope("writer-outlives-signal", || {
        let mut tasks = AsyncRuntime::new().expect("build tokio runtime");
        tasks.install_current();
        let gone = Owner::scope("child", || crate::reactive::signal(0));
        let writer = gone.writer();
        writer.set(7).expect("send");
        writer.update(|value| *value += 1).expect("send");
        let _ = tasks.pump();
    });
}
