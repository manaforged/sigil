#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use sigil::hal::Scheduler;
use sigil::reactive::{Owner, Runtime};
use sigil::tasks::{AsyncRuntime, OwnerHandle};

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn setup() -> (AsyncRuntime, OwnerHandle) {
    let rt = AsyncRuntime::new().expect("build tokio runtime");
    let handle = OwnerHandle::current().expect("inside a reactive scope");
    (rt, handle)
}

#[must_use]
pub fn spin_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
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
fn enqueue_main_runs_callback_on_drain() {
    Runtime::scope("enqueue_main_runs_callback_on_drain", || {
        let (mut scheduler, handle) = setup();
        let ran = Arc::new(AtomicBool::new(false));

        let ran_cb = Arc::clone(&ran);
        scheduler
            .enqueue_main(
                handle,
                Box::new(move || {
                    ran_cb.store(true, Ordering::SeqCst);
                }),
            )
            .expect("enqueue");

        assert!(
            !ran.load(Ordering::SeqCst),
            "callback must not run until drain"
        );
        let drained = scheduler.drain();
        assert_eq!(drained, 1, "one callback should have been drained");
        assert!(
            ran.load(Ordering::SeqCst),
            "callback ran after drain on main thread"
        );
    });
}

#[test]
fn enqueue_main_drops_callback_for_disposed_owner() {
    let ran = Arc::new(AtomicBool::new(false));
    let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");

    let owner = Owner::new("enqueue_main_drops_callback_for_disposed_owner", None);
    let handle = OwnerHandle::from_id(owner.id());
    let ran_cb = Arc::clone(&ran);
    scheduler
        .enqueue_main(
            handle,
            Box::new(move || {
                ran_cb.store(true, Ordering::SeqCst);
            }),
        )
        .expect("enqueue");
    owner.dispose();

    let drained = scheduler.drain();
    assert_eq!(drained, 0, "disposed-owner callback must be dropped");
    assert!(
        !ran.load(Ordering::SeqCst),
        "callback must not run for disposed owner"
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn wake_proxy_is_called_on_enqueue() {
    Runtime::scope("wake_proxy_is_called_on_enqueue", || {
        let (mut scheduler, handle) = setup();
        let wakes = Arc::new(AtomicUsize::new(0));
        {
            let wakes = Arc::clone(&wakes);
            scheduler.set_waker(move || {
                wakes.fetch_add(1, Ordering::SeqCst);
            });
        }

        for _ in 0..3 {
            scheduler
                .enqueue_main(handle, Box::new(|| {}))
                .expect("enqueue");
        }

        assert_eq!(
            wakes.load(Ordering::SeqCst),
            3,
            "waker fires once per successful enqueue"
        );

        scheduler.drain();
    });
}
