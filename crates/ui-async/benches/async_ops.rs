//! Tier 3 benches for the `ui-async` cross-thread runtime.
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
//! cargo bench -p ui-async --bench async_ops -- \
//!     --warm-up-time 1 --measurement-time 3 --sample-size 30
//! ```

#![allow(missing_docs, clippy::missing_docs_in_private_items)]
// Bench targets are allowed to bend `panic` / `indexing_slicing` per
// the rust-hygiene framework (docs/plans/native-ui/hygiene-cleanup-framework.md).
#![allow(clippy::panic)]

// `winit` is a regular dep on `ui-async` (event-loop wiring) but is
// not used by this bench binary; silence the unused-dep lint that
// flows into every bench compilation unit.
use winit as _;

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ui_async::{AsyncRuntime, OwnerHandle, dispatch_to_owner, spawn_to_signal};
use ui_hal::Scheduler;
use ui_reactive::{Runtime, effect, signal};

/// Spin until `cond` returns true or the deadline elapses.
/// Used by the round-trip benches to wait for cross-thread work to
/// land in the dispatcher queue without depending on `tokio::time`.
fn spin_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
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
                // Drain to keep the queue from growing unbounded
                // across iterations. Drain cost is cheap for a
                // single empty callback; this still leaves
                // `enqueue_main` as the dominant measured op.
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
// drains, and reads back a signal-driven flag. Measures wake + drain +
// callback + signal write end-to-end.
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
                // Spawn one tokio task that immediately enqueues a
                // main-thread closure. Use the scheduler trait to
                // spawn so abort registry stays sane.
                scheduler.spawn(handle, async move {
                    dispatch_to_owner(&dispatcher_cb, handle, move || {
                        observed_cb.fetch_add(1, Ordering::Relaxed);
                    });
                });
                // Wait for the cross-thread send to land. The atomic
                // flip happens inside the waker callback, so observing
                // `woke == true` means the message is already on the
                // queue and ready to drain.
                assert!(
                    spin_until(Duration::from_millis(50), || woke.load(Ordering::Acquire)),
                    "dispatch did not land within 50ms"
                );
                let drained = scheduler.drain();
                black_box(drained);
            });
            // Sanity: at least one round-trip observed.
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
//
// Pre-enqueues 100 no-op callbacks via `iter_batched` setup, then
// measures `drain()` only. Measures the inner loop (try_recv + owner
// liveness check + box drop) without any payload work.
fn bench_drain_100_empty_callbacks(c: &mut Criterion) {
    c.bench_function("drain_100_empty_callbacks", |b| {
        Runtime::scope("bench_drain_100_empty", || {
            // One scheduler shared across iterations; drain leaves it
            // empty so the next setup starts from a clean slate.
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
// No DR-009 budget — scaling probe. Ten times the previous workload.
// Compare with `drain_100_empty_callbacks` to see whether per-callback
// drain cost is roughly linear (which it should be for a try_recv
// loop) or whether some sub-linear effect dominates.
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
//
// 100 callbacks each write to a distinct signal, each signal has one
// effect subscribed. The drain therefore performs 100 callback runs
// + 100 signal writes + 100 effect re-runs. This is the realistic
// "background data lands and triggers re-renders" workload.
fn bench_drain_100_callbacks_each_writing_signal(c: &mut Criterion) {
    c.bench_function("drain_100_callbacks_each_writing_signal", |b| {
        Runtime::scope("bench_drain_100_signals", || {
            // Build 100 signals + effects up front. Each effect just
            // observes its signal so the write-path runs the effect
            // exactly once per drained callback.
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

            // Move the signal vec into the snapshot — clippy won't
            // accept a `.clone()` since the original isn't reused.
            let signals_snapshot = signals;
            b.iter_batched(
                || {
                    let sched = scheduler_cell.borrow();
                    for (i, sig) in signals_snapshot.iter().copied().enumerate() {
                        // `Signal<T>` is `!Send`, but `enqueue_main`'s
                        // boxed callback must satisfy `Send` at the
                        // type level. Shuttle through `SendableSignal`
                        // (the cheap identity wrapper) and re-hydrate
                        // on the main thread inside the callback.
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

// ---------------------------------------------------------------------------
// 6. sustained_cross_thread_writes_1k_per_sec
// ---------------------------------------------------------------------------
//
// DR-009 budget: sustained 1k writes/sec, frame latency p99 ≤ 16 ms.
//
// A background tokio task writes a signal at ~1 kHz via
// `dispatch_to_owner`. The main thread simulates a 60 Hz frame loop:
// every ~16.6 ms it drains the queue and records the drain duration.
// We collect roughly 60 frames (one second of simulated rendering)
// per criterion iteration via `iter_custom`, then divide by the
// number of frames to report a per-frame mean. The per-frame max is
// also asserted to stay under the 16 ms budget; if it doesn't, the
// bench panics with the offending value.
//
// `iter_custom` lets us measure the wall-clock of one full 1-second
// run rather than letting criterion's sample-count balloon the total
// runtime. We tell criterion `sample-size=10` via the bench config
// and a single iteration of this fn already does ~60 frames worth
// of work.
fn bench_sustained_cross_thread_writes_1k_per_sec(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_cross_thread_writes_1k_per_sec");
    // One iteration of this bench is already ~1 s of wall clock; cap
    // the sample size so total runtime stays bounded.
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(12));
    group.warm_up_time(Duration::from_secs(1));
    group.bench_function("frame_drain_under_1khz_writes", |b| {
        b.iter_custom(|iters| {
            // Per-iteration: spin up a fresh runtime + writer, run
            // the simulated frame loop for ~1 s, tear down.
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let elapsed = Runtime::scope("bench_sustained_writes", run_one_sustained_iteration);
                total += elapsed;
            }
            total
        });
    });
    group.finish();
}

/// One iteration of the sustained-writes bench. Returns the sum of
/// per-frame drain durations across one simulated second.
///
/// Split out of the closure body so clippy's `too_many_lines` /
/// nesting lints don't fire on the bench function itself.
fn run_one_sustained_iteration() -> Duration {
    let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");
    // Waker is a no-op for this bench: the main thread drives
    // drain on its own cadence. The real shell would use the
    // waker to coalesce winit redraws.
    let handle = OwnerHandle::current().expect("inside a reactive scope");
    let dispatcher = scheduler.dispatcher();
    let tick = signal(0_u64);
    let tick_sendable = tick.into_sendable();

    // Background task: fire one write per millisecond paced against
    // `Instant::now()`, yielding to the tokio scheduler between
    // checks. We avoid `tokio::time::sleep` so the workspace tokio
    // doesn't need the `time` feature flag enabled. Pacing accuracy
    // is ~10s of µs which is plenty for a "sustained 1 kHz" probe.
    let stop = Arc::new(AtomicBool::new(false));
    let stop_writer = Arc::clone(&stop);
    let writer_started = Instant::now();
    scheduler.spawn(handle, async move {
        let mut counter = 0_u64;
        let period = Duration::from_micros(1000);
        let mut next_at = writer_started + period;
        while !stop_writer.load(Ordering::Relaxed) {
            // Spin-yield until we hit the next 1 ms tick.
            while Instant::now() < next_at {
                tokio::task::yield_now().await;
                if stop_writer.load(Ordering::Relaxed) {
                    return;
                }
            }
            counter = counter.wrapping_add(1);
            let dispatcher = dispatcher.clone();
            dispatch_to_owner(&dispatcher, handle, move || {
                let s = tick_sendable.into_signal();
                s.set(counter);
            });
            next_at += period;
        }
    });

    // Simulated 60 Hz frame loop for ~1 second.
    let total_frames = 60_u32;
    let frame_period = Duration::from_micros(16_667);
    let mut sum = Duration::ZERO;
    let mut max = Duration::ZERO;
    let loop_start = Instant::now();
    for frame in 0..total_frames {
        let next_frame_at = loop_start + frame_period * (frame + 1);
        let drain_start = Instant::now();
        let drained = scheduler.drain();
        let drain_dur = drain_start.elapsed();
        black_box(drained);
        sum += drain_dur;
        if drain_dur > max {
            max = drain_dur;
        }
        // Sleep until the next frame boundary to keep the cadence
        // honest. If the drain itself overshot the period, do not
        // sleep — start the next frame immediately.
        let now = Instant::now();
        if now < next_frame_at {
            std::thread::sleep(next_frame_at - now);
        }
    }
    stop.store(true, Ordering::Relaxed);

    // Final drain to flush whatever the writer enqueued after the
    // last frame, before tearing the runtime down.
    let _ = scheduler.drain();

    // Hard budget check on the worst frame. 16.67 ms is the 60 Hz
    // frame budget; any single drain that blows it would mean the
    // background task starved the frame.
    assert!(
        max <= Duration::from_millis(16),
        "frame-drain max {max:?} exceeded 16 ms budget under 1 kHz writes"
    );
    sum
}

// ---------------------------------------------------------------------------
// 7. spawn_to_signal_smoke_latency
// ---------------------------------------------------------------------------
//
// No fixed DR-009 budget. Measures `spawn_to_signal` overhead with an
// already-resolved future: signal construction + tokio spawn +
// dispatch + drain + signal read. Useful as a "cost of one fetch"
// floor; product code that issues many fetches per frame should
// stay well under any budget here.
fn bench_spawn_to_signal_smoke_latency(c: &mut Criterion) {
    c.bench_function("spawn_to_signal_smoke_latency", |b| {
        Runtime::scope("bench_spawn_to_signal", || {
            let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            b.iter(|| {
                let sig = spawn_to_signal::<u32, &'static str, _>(&scheduler, handle, async {
                    Ok(42_u32)
                });
                // Wait for the result to land in the queue, then
                // drain. The future resolves immediately on a tokio
                // worker so the wait is short.
                let pending = || scheduler.pending_count() >= 1;
                assert!(
                    spin_until(Duration::from_millis(100), pending),
                    "spawn_to_signal did not enqueue within 100ms"
                );
                let drained = scheduler.drain();
                black_box(drained);
                let v = sig.get();
                assert!(v.is_ok(), "signal should resolve to Ok");
                black_box(v);
            });
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
