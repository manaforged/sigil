//! Integration tests for the `sigil-async` scheduler adapter — enqueue/drain
//! and owner-scoping invariants.
//!
//! Spawn/cancellation and `spawn_to_signal` tests live in
//! `scheduler_spawn.rs`.
//!
//! Each test exercises one invariant from [DR-007] and [DR-005
//! addendum §3] end-to-end: enqueue-order preservation, owner-disposal
//! cancellation, waker firing, and the reactive-owner scoping of
//! `dispatch_to_owner` callbacks. We keep the surface primitive by
//! leaning on atomic flags and short polling loops instead of
//! `tokio::time`; that way the `time` feature stays off and the
//! tests do not flake on a slow runner.
//!
//! Tests are self-contained — each creates its own reactive scope via
//! `Runtime::scope` and its own `AsyncRuntime`. That keeps them
//! independent despite the thread-local reactive arena.
//!
//! [DR-007]: ../../../docs/decisions/DR-007-async-tokio.md
//! [DR-005 addendum §3]: ../../../docs/decisions/DR-005-reactivity-owner-tree.md

// Integration-test file: tests assert by panicking. rust-hygiene
// contract permits panic / indexing_slicing bends in test code via
// crate-root #![allow]. See docs/plans/native-ui/hygiene-cleanup-framework.md.
#![allow(clippy::panic)]
#![allow(missing_docs)]

// `winit` is a regular dep on `sigil-async` (future event-loop-proxy
// wiring) but is not used by any of these tests; silence the
// unused-dep lint that flows into every integration-test binary.
use winit as _;
// `criterion` is a dev-dep used only by `benches/async_ops.rs`. Cargo
// pulls every dev-dep into every test binary, which trips
// `unused_crate_dependencies` here. Alias silences the lint without a
// real import.
use criterion as _;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use sigil_hal::Scheduler;
use sigil_reactive::{Owner, Runtime};
use sigil_tasks::{AsyncRuntime, OwnerHandle, dispatch_to_owner};

/// Small helper: create an `AsyncRuntime` + capture the current owner
/// as an `OwnerHandle`. Called inside a `Runtime::scope` closure.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn setup() -> (AsyncRuntime, OwnerHandle) {
    let rt = AsyncRuntime::new().expect("build tokio runtime");
    let handle = OwnerHandle::current().expect("inside a reactive scope");
    (rt, handle)
}

/// Poll the closure at 1 ms intervals until it returns `true` or the
/// deadline elapses. Returns `true` iff the predicate succeeded.
///
/// Used to wait for cross-thread state transitions without pulling in
/// `tokio::time` (which would require enabling its `time` feature).
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
    // Explicit owner so we can dispose while the scheduler is still
    // alive and exercise the dead-owner drain path in the same test
    // body. `Runtime::scope` disposes on exit, which would also work,
    // but this form is clearer.
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

// Three chained tasks in deterministic order make this test inherently
// sequential — splitting would fragment the narrative without reducing
// complexity. Same rationale as `builds_expected_tree_shape`.
#[allow(clippy::too_many_lines)]
#[test]
fn drain_preserves_enqueue_order() {
    Runtime::scope("drain_preserves_enqueue_order", || {
        let (mut scheduler, handle) = setup();
        let order: Arc<std::sync::Mutex<Vec<&'static str>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        // Three background tasks each push one label. Tasks chain
        // through atomic gates so the enqueue-order is deterministic
        // — a, then b, then c — no matter which tokio worker picked
        // each task up.
        let gate_a = Arc::new(AtomicBool::new(false));
        let gate_b = Arc::new(AtomicBool::new(false));
        let gate_c = Arc::new(AtomicBool::new(false));

        let dispatcher = scheduler.dispatcher();

        // Task A
        {
            let dispatcher = dispatcher.clone();
            let order = Arc::clone(&order);
            let gate_a = Arc::clone(&gate_a);
            scheduler.spawn(handle, async move {
                dispatch_to_owner(&dispatcher, handle, move || {
                    order.lock().expect("mutex").push("a");
                });
                gate_a.store(true, Ordering::SeqCst);
            });
        }
        // Task B waits for A, then enqueues.
        {
            let dispatcher = dispatcher.clone();
            let order = Arc::clone(&order);
            let gate_a_rd = Arc::clone(&gate_a);
            let gate_b = Arc::clone(&gate_b);
            scheduler.spawn(handle, async move {
                while !gate_a_rd.load(Ordering::SeqCst) {
                    tokio::task::yield_now().await;
                }
                dispatch_to_owner(&dispatcher, handle, move || {
                    order.lock().expect("mutex").push("b");
                });
                gate_b.store(true, Ordering::SeqCst);
            });
        }
        // Task C waits for B, then enqueues. (Last task consumes
        // the dispatcher directly — no final clone needed.)
        {
            let order = Arc::clone(&order);
            let gate_b_rd = Arc::clone(&gate_b);
            let gate_c = Arc::clone(&gate_c);
            scheduler.spawn(handle, async move {
                while !gate_b_rd.load(Ordering::SeqCst) {
                    tokio::task::yield_now().await;
                }
                dispatch_to_owner(&dispatcher, handle, move || {
                    order.lock().expect("mutex").push("c");
                });
                gate_c.store(true, Ordering::SeqCst);
            });
        }

        // All three should have enqueued within a generous deadline.
        assert!(
            spin_until(Duration::from_secs(5), || gate_c.load(Ordering::SeqCst)),
            "tasks a, b, c should all enqueue"
        );

        let drained = scheduler.drain();
        assert_eq!(drained, 3, "three callbacks should have drained");
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
            let observed: Arc<std::sync::Mutex<Option<sigil_reactive::OwnerId>>> =
                Arc::new(std::sync::Mutex::new(None));
            let observed_cb = Arc::clone(&observed);

            let dispatcher = scheduler.dispatcher();
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

#[test]
fn drain_does_not_process_callbacks_enqueued_during_drain() {
    // Proves the Finding #3 fix: drain honours a snapshot of the
    // queue length at drain-start. A callback that re-enqueues
    // itself (or enqueues another callback) during drain must NOT
    // be processed in the same drain cycle.
    Runtime::scope(
        "drain_does_not_process_callbacks_enqueued_during_drain",
        || {
            let (mut scheduler, handle) = setup();
            let a_ran = Arc::new(AtomicBool::new(false));
            let b_ran = Arc::new(AtomicBool::new(false));

            // Callback A, when it runs, enqueues callback B through the
            // dispatcher. If drain were unbounded it would keep reading
            // until empty and run B in the same cycle.
            let dispatcher_for_a = scheduler.dispatcher();
            let a_ran_cb = Arc::clone(&a_ran);
            let b_ran_for_b = Arc::clone(&b_ran);
            scheduler
                .enqueue_main(
                    handle,
                    Box::new(move || {
                        a_ran_cb.store(true, Ordering::SeqCst);
                        // Enqueue B from inside A — this is the re-entry
                        // that used to starve the frame.
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

            // A second drain picks up B.
            let ran_second = scheduler.drain();
            assert_eq!(ran_second, 1, "second drain runs exactly B");
            assert!(
                b_ran.load(Ordering::SeqCst),
                "B runs on the next drain cycle"
            );
        },
    );
}
