#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

//! Tier 3 real-time benches for `sigil-reactive`.
//!
//! Sustained / burst write throughput shapes that mirror live-data
//! UIs: dashboards streaming WebSocket updates, virtualized grids
//! re-binding cells per frame, theme switches under live data.
//!
//! Targets from DR-009 §2:
//!
//! - Sustained 1k writes/sec, frame latency p99   → ≤ 16 ms
//! - Burst 10k writes batched (one frame's worth) → ≤  5 ms
//!
//! Cross-thread dispatch round-trip is in `sigil-async`'s scope, not
//! here.
//!
//! ## Per-frame percentile note
//!
//! Criterion measures the whole `b.iter` body, not slices within it.
//! For the "sustained N frames" benches, each sample is one full
//! frame-budget block (e.g. 100 frames × 1000 writes); the reported
//! median/p95 are over those whole-block samples, not over individual
//! frames. To assert "per-frame p99 ≤ 16 ms" on a single frame, divide
//! the median total by the frame count and treat the result as the
//! steady-state per-frame number; criterion's noise band (±a few %)
//! is well below the budget headroom.
//!
//! Run with `cargo bench -p sigil-reactive --bench realtime`. Use
//! `-- --warm-up-time 1 --measurement-time 3 --sample-size 30` for
//! the dev-loop numbers.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_reactive::{Runtime, batch, effect, signal};

/// Budget: ≤ 5 ms per batch (DR-009 burst).
///
/// 10 000 writes to one signal inside one `batch`. The subscribed
/// effect must run exactly once at drain. Measures coalescing
/// throughput on the simplest possible shape: one signal, one
/// subscriber, write-storm.
fn bench_burst_10k_writes_single_signal_coalesced(c: &mut Criterion) {
    c.bench_function("burst_10k_writes_single_signal_coalesced", |b| {
        Runtime::scope("bench_burst_single", || {
            let n = signal(0_i32);
            let sink = Rc::new(Cell::new(0_i32));
            let sink_for_effect = Rc::clone(&sink);
            effect(move || sink_for_effect.set(n.get()));
            b.iter(|| {
                batch(|| {
                    for i in 0..10_000_i32 {
                        n.set(black_box(i));
                    }
                });
            });
        });
    });
}

/// Budget: ≤ 5 ms per batch (DR-009 burst).
///
/// 10 000 distinct signals, one effect that reads them all (10 000
/// dependencies). Inside one `batch`, write each signal once. The
/// effect re-runs once at drain and re-walks all 10 000 deps.
///
/// Stress test for deps management under burst — exercises the
/// per-run dep-clear and re-subscribe paths in `effect::run_setup`.
fn bench_burst_10k_writes_distinct_signals_one_effect(c: &mut Criterion) {
    c.bench_function("burst_10k_writes_distinct_signals_one_effect", |b| {
        Runtime::scope("bench_burst_distinct", || {
            let mut signals = Vec::with_capacity(10_000);
            for _ in 0..10_000 {
                signals.push(signal(0_i32));
            }
            let signals_for_effect = signals.clone();
            let sink = Rc::new(Cell::new(0_i32));
            let sink_for_effect = Rc::clone(&sink);
            effect(move || {
                let mut acc = 0_i32;
                for s in &signals_for_effect {
                    acc = acc.wrapping_add(s.get());
                }
                sink_for_effect.set(acc);
            });
            b.iter(|| {
                batch(|| {
                    let mut value = 0_i32;
                    for s in &signals {
                        value = value.wrapping_add(1);
                        s.set(black_box(value));
                    }
                });
            });
        });
    });
}

/// Budget: per-frame p99 ≤ 16 ms (DR-009 sustained).
///
/// 100 frame ticks, each with 1000 writes to one signal subscribed
/// by one effect. Writes in each frame are NOT batched — this is
/// the "stream of independent updates arriving at frame rate" shape
/// rather than "one frame's coalesced update". Each `set` runs the
/// effect synchronously.
///
/// Reported number is the median total for 100 frames × 1000 writes.
/// Divide by 100 for the per-frame steady-state cost; that number
/// must stay below 16 ms to hit the budget.
fn bench_sustained_writes_1000_per_frame_100_frames(c: &mut Criterion) {
    c.bench_function("sustained_writes_1000_per_frame_100_frames", |b| {
        Runtime::scope("bench_sustained_1k", || {
            let n = signal(0_i32);
            let sink = Rc::new(Cell::new(0_i32));
            let sink_for_effect = Rc::clone(&sink);
            effect(move || sink_for_effect.set(n.get()));
            b.iter(|| {
                let mut counter = 0_i32;
                for _frame in 0..100 {
                    for _ in 0..1000 {
                        counter = counter.wrapping_add(1);
                        n.set(black_box(counter));
                    }
                }
            });
        });
    });
}

/// Budget: per-frame p99 ≤ 16 ms (DR-009 sustained).
///
/// "Theme flicker under live data" worst case: 1000 subscribers, 100
/// writes per frame, 10 frames. Each write fires every subscriber
/// synchronously, so the per-frame cost is dominated by the 100 ×
/// 1000 = 100 000 effect invocations.
///
/// Reported number is the median total for 10 frames. Divide by 10
/// for per-frame; budget is 16 ms per frame.
fn bench_sustained_writes_fanout_1k_subscribers_10_frames(c: &mut Criterion) {
    c.bench_function("sustained_writes_fanout_1k_subscribers_10_frames", |b| {
        Runtime::scope("bench_sustained_fanout", || {
            let n = signal(0_i32);
            // Effects own their own `Rc<Cell<i32>>` clone via the
            // closure capture; we don't need to keep the originating
            // side alive on the bench frame. Each effect is registered
            // against the enclosing `Runtime::scope` and lives until
            // the scope is disposed, regardless of what the outer
            // bench code holds.
            for _ in 0..1000 {
                let sink: Rc<Cell<i32>> = Rc::new(Cell::new(0_i32));
                effect(move || sink.set(n.get()));
            }
            b.iter(|| {
                let mut counter = 0_i32;
                for _frame in 0..10 {
                    for _ in 0..100 {
                        counter = counter.wrapping_add(1);
                        n.set(black_box(counter));
                    }
                }
            });
        });
    });
}

/// Steady-state read/write throughput. Alternating `.set(n)` /
/// `.get()` pairs, 10 000 iterations per criterion sample. No
/// subscribers, so this is the pure single-thread arena round-trip
/// cost — the optimistic upper bound on what a real workload can
/// achieve.
///
/// Expectation per DR-009 microbench numbers (`signal_get` ≤ 10 ns,
/// `signal_set` no-sub ≤ ~120 ns floor): ~85-130 ns per pair, so
/// ~7-12 M ops/sec. The bench reports total time for 10 000 pairs.
fn bench_write_then_read_throughput(c: &mut Criterion) {
    c.bench_function("write_then_read_throughput", |b| {
        Runtime::scope("bench_throughput", || {
            let n = signal(0_i32);
            let mut counter = 0_i32;
            b.iter(|| {
                let mut acc = 0_i32;
                for _ in 0..10_000 {
                    counter = counter.wrapping_add(1);
                    n.set(black_box(counter));
                    acc = acc.wrapping_add(black_box(n.get()));
                }
                black_box(acc);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_burst_10k_writes_single_signal_coalesced,
    bench_burst_10k_writes_distinct_signals_one_effect,
    bench_sustained_writes_1000_per_frame_100_frames,
    bench_sustained_writes_fanout_1k_subscribers_10_frames,
    bench_write_then_read_throughput,
);
criterion_main!(benches);
