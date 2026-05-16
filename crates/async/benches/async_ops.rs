//! Tier 3 benches for the `sigil-async` cross-thread runtime.
//!
//! DR-009 budgets exercised here:
//!
//! - Cross-thread dispatch round-trip ≤ 500 µs p99
//! - Drain with 100 pending callbacks ≤ 2 ms
//! - Sustained 1k writes/sec, frame latency p99 ≤ 16 ms
//!
//! Each bench is annotated inline with its DR-009 line and target.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p sigil-async --bench async_ops -- \
//!     --warm-up-time 1 --measurement-time 3 --sample-size 30
//! ```
//!
//! Sustained-writes and `spawn_to_signal` benches live in `async_ops_sustained`.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]
// Bench targets are allowed to bend `panic` / `indexing_slicing` per
// the rust-hygiene framework (docs/plans/native-ui/hygiene-cleanup-framework.md).
#![allow(clippy::panic)]

mod async_ops_sustained;

// `winit` is a regular dep on `sigil-async` (event-loop wiring) but is
// not used by this bench binary; silence the unused-dep lint that
// flows into every bench compilation unit.
use winit as _;

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_ops_sustained::{
    bench_spawn_to_signal_smoke_latency, bench_sustained_cross_thread_writes_1k_per_sec,
};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil_async::{AsyncRuntime, OwnerHandle, dispatch_to_owner};
use sigil_hal::Scheduler;
use sigil_reactive::{Runtime, effect, signal};

/// Spin until `cond` returns true or the deadline elapses.
/// Used by the round-trip benches to wait for cross-thread work to
/// land in the dispatcher queue without depending on `tokio::time`.
pub(crate) fn spin_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if cond() {
            return true;
        }
        std::thread::yield_now();
    }
    cond()
}

// ---------------------------------------------------------------------------
// 1. enqueue_main_latency
// ---------------------------------------------------------------------------
//
// Floor cost for a single MPSC send + waker fire. Same-thread (no hop),
// no waker installed. Expected: tens of ns. This is the lower bound on
// any cross-thread enqueue — anything above this is wake / scheduler
// overhead.
fn bench_enqueue_main_latency(c: &mut Criterion) {
    c.bench_function("enqueue_main_latency", |b| {
        Runtime::scope("bench_enqueue_main", || {
            let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            b.iter(|| {
                scheduler
                    .enqueue_main(handle, Box::new(|| {}))
                    .expect("enqueue");
                let _ = scheduler.drain();
            });
        });
    });
}

// ---------------------------------------------------------------------------
// 2. dispatch_to_owner_roundtrip_single
// ---------------------------------------------------------------------------
//
// DR-009 budget: cross-thread dispatch round-trip ≤ 500 µs p99.
//
// Full path: tokio worker calls `dispatch_to_owner`, the dispatcher
// flips an atomic via the waker, the main thread spins until pending,
// drains, and reads back a signal-driven flag.
fn bench_dispatch_to_owner_roundtrip_single(c: &mut Criterion) {
    c.bench_function("dispatch_to_owner_roundtrip_single", |b| {
        Runtime::scope("bench_dispatch_roundtrip", || {
            let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            let woke = Arc::new(AtomicBool::new(false));
            {
                let woke = Arc::clone(&woke);
                scheduler.set_waker(move || {
                    woke.store(true, Ordering::Release);
                });
            }
            let dispatcher = scheduler.dispatcher();
            let observed = Arc::new(AtomicUsize::new(0));
            b.iter(|| {
                woke.store(false, Ordering::Release);
                let dispatcher_cb = dispatcher.clone();
                let observed_cb = Arc::clone(&observed);
                scheduler.spawn(handle, async move {
                    dispatch_to_owner(&dispatcher_cb, handle, move || {
                        observed_cb.fetch_add(1, Ordering::Relaxed);
                    });
                });
                assert!(
                    spin_until(Duration::from_millis(50), || woke.load(Ordering::Acquire)),
                    "dispatch did not land within 50ms"
                );
                let drained = scheduler.drain();
                black_box(drained);
            });
            assert!(
                observed.load(Ordering::Relaxed) > 0,
                "expected at least one roundtrip to land"
            );
        });
    });
}

// ---------------------------------------------------------------------------
// 3. drain_100_empty_callbacks
// ---------------------------------------------------------------------------
//
// DR-009 budget: drain with 100 pending callbacks ≤ 2 ms.
fn bench_drain_100_empty_callbacks(c: &mut Criterion) {
    c.bench_function("drain_100_empty_callbacks", |b| {
        Runtime::scope("bench_drain_100_empty", || {
            let scheduler_cell =
                std::cell::RefCell::new(AsyncRuntime::new().expect("build tokio runtime"));
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            b.iter_batched(
                || {
                    let sched = scheduler_cell.borrow();
                    for _ in 0..100 {
                        sched
                            .enqueue_main(handle, Box::new(|| {}))
                            .expect("pre-enqueue");
                    }
                    drop(sched);
                },
                |()| {
                    let mut sched = scheduler_cell.borrow_mut();
                    let drained = sched.drain();
                    black_box(drained);
                },
                BatchSize::SmallInput,
            );
        });
    });
}

// ---------------------------------------------------------------------------
// 4. drain_1000_empty_callbacks
// ---------------------------------------------------------------------------
//
// No DR-009 budget — scaling probe.
fn bench_drain_1000_empty_callbacks(c: &mut Criterion) {
    c.bench_function("drain_1000_empty_callbacks", |b| {
        Runtime::scope("bench_drain_1000_empty", || {
            let scheduler_cell =
                std::cell::RefCell::new(AsyncRuntime::new().expect("build tokio runtime"));
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            b.iter_batched(
                || {
                    let sched = scheduler_cell.borrow();
                    for _ in 0..1_000 {
                        sched
                            .enqueue_main(handle, Box::new(|| {}))
                            .expect("pre-enqueue");
                    }
                    drop(sched);
                },
                |()| {
                    let mut sched = scheduler_cell.borrow_mut();
                    let drained = sched.drain();
                    black_box(drained);
                },
                BatchSize::SmallInput,
            );
        });
    });
}

// ---------------------------------------------------------------------------
// 5. drain_100_callbacks_each_writing_signal
// ---------------------------------------------------------------------------
//
// DR-009 budget: drain with 100 pending callbacks ≤ 2 ms (with payload).
fn bench_drain_100_callbacks_each_writing_signal(c: &mut Criterion) {
    c.bench_function("drain_100_callbacks_each_writing_signal", |b| {
        Runtime::scope("bench_drain_100_signals", || {
            let signals: Vec<_> = (0..100).map(|_| signal(0_i32)).collect();
            for s in &signals {
                let s = *s;
                effect(move || {
                    black_box(s.get());
                });
            }
            let scheduler_cell =
                std::cell::RefCell::new(AsyncRuntime::new().expect("build tokio runtime"));
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            let signals_snapshot = signals;
            b.iter_batched(
                || {
                    let sched = scheduler_cell.borrow();
                    for (i, sig) in signals_snapshot.iter().copied().enumerate() {
                        let sendable = sig.into_sendable();
                        let value = i32::try_from(i).expect("loop bound fits in i32");
                        sched
                            .enqueue_main(
                                handle,
                                Box::new(move || {
                                    let s = sendable.into_signal();
                                    s.set(value);
                                }),
                            )
                            .expect("pre-enqueue");
                    }
                    drop(sched);
                },
                |()| {
                    let mut sched = scheduler_cell.borrow_mut();
                    let drained = sched.drain();
                    black_box(drained);
                },
                BatchSize::SmallInput,
            );
        });
    });
}

criterion_group!(
    benches,
    bench_enqueue_main_latency,
    bench_dispatch_to_owner_roundtrip_single,
    bench_drain_100_empty_callbacks,
    bench_drain_1000_empty_callbacks,
    bench_drain_100_callbacks_each_writing_signal,
    bench_sustained_cross_thread_writes_1k_per_sec,
    bench_spawn_to_signal_smoke_latency,
);
criterion_main!(benches);
