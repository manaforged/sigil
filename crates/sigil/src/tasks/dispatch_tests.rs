#![allow(clippy::panic, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::hal::Scheduler;
use crate::reactive::{Owner, Runtime};
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::{AsyncRuntime, OwnerHandle};

fn setup() -> (AsyncRuntime, OwnerHandle) {
    let rt = AsyncRuntime::new().expect("build tokio runtime");
    let handle = OwnerHandle::current().expect("inside a reactive scope");
    (rt, handle)
}

fn spin_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    cond()
}

#[test]
fn drain_preserves_enqueue_order() {
    Runtime::scope("drain_preserves_enqueue_order", || {
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
        assert_eq!(scheduler.drain(), 3, "three callbacks should have drained");
        assert_eq!(
            *order.lock().expect("mutex"),
            vec!["a", "b", "c"],
            "drain preserves enqueue order"
        );
    });
}

#[test]
fn dispatch_to_owner_sets_current_owner_during_callback() {
    Runtime::scope(
        "dispatch_to_owner_sets_current_owner_during_callback",
        || {
            let (mut scheduler, handle) = setup();
            let expected = handle.id();
            let observed: Arc<std::sync::Mutex<Option<crate::reactive::OwnerId>>> =
                Arc::new(std::sync::Mutex::new(None));
            let observed_cb = Arc::clone(&observed);

            let dispatcher = scheduler.spawn_parts().dispatcher;
            dispatch_to_owner(&dispatcher, handle, move || {
                *observed_cb.lock().expect("mutex") = Owner::current();
            });

            scheduler.drain();

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
    Runtime::scope(
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

            let ran_first = scheduler.drain();
            assert_eq!(ran_first, 1, "first drain runs exactly A (budget == 1)");
            assert!(a_ran.load(Ordering::SeqCst), "A ran");
            assert!(
                !b_ran.load(Ordering::SeqCst),
                "B was enqueued during drain and must defer to the next cycle"
            );

            let ran_second = scheduler.drain();
            assert_eq!(ran_second, 1, "second drain runs exactly B");
            assert!(
                b_ran.load(Ordering::SeqCst),
                "B runs on the next drain cycle"
            );
        },
    );
}
