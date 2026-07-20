//! Tier 2 fan-out benches for `sigil-reactive`.
//!
//! These cover the "one write notifies N dependents" shape that
//! drives theme switches, locale changes, broadcast events, and any
//! UI state owned by a hub-and-spokes signal.
//!
//! Targets from DR-009 §2:
//!
//! - `signal.set()` with 100 subs   → ≤ 20 µs
//! - `signal.set()` with 1000 subs  → ≤ 200 µs
//!
//! 10 000-subscriber and chain-depth-100 are stress shapes — measure
//! and record, no fixed budget.
//!
//! Run with `cargo bench -p sigil-reactive --bench fanout`. Use
//! `-- --warm-up-time 1 --measurement-time 3 --sample-size 30` for
//! the dev-loop numbers; CI smoke uses shorter windows.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

// `actions`, `smallvec`, and `tokio` are regular/dev deps pulled
// into every bench compilation unit. We don't reference them directly
// here, so silence `unused_crate_dependencies` with marker aliases.
use sigil as _;
use sigil_actions as _;
use smallvec as _;
use tokio as _;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil_reactive::{Runtime, derived, effect, signal};

/// Build `count` effects that all read `sig` and store the value into
/// their own `Cell`. The sinks are returned so the caller can keep
/// them alive across the timed section — dropping them inside the
/// closure would skew the measurement and let the optimiser fold the
/// effect bodies away.
fn install_subscribers(sig: sigil_reactive::Signal<i32>, count: usize) -> Vec<Rc<Cell<i32>>> {
    let mut sinks = Vec::with_capacity(count);
    for _ in 0..count {
        let sink = Rc::new(Cell::new(0_i32));
        let sink_for_effect = Rc::clone(&sink);
        effect(move || sink_for_effect.set(sig.get()));
        sinks.push(sink);
    }
    sinks
}

/// Budget: ≤ 20 µs (DR-009).
///
/// One signal, 100 subscribed effects, time one `.set()` (which clones
/// the 100-entry subscriber set and runs each effect once).
fn bench_set_100_subscribers(c: &mut Criterion) {
    c.bench_function("signal_set_100_subscribers", |b| {
        Runtime::scope("bench_fanout_100", || {
            let n = signal(0_i32);
            let _sinks = install_subscribers(n, 100);
            let mut i = 0_i32;
            b.iter(|| {
                i = i.wrapping_add(1);
                n.set(black_box(i));
            });
        });
    });
}

/// Budget: ≤ 200 µs (DR-009).
///
/// 1000 subscribers — the broadcast / theme-switch shape. Five of
/// these in one frame must still fit in 16 ms.
fn bench_set_1000_subscribers(c: &mut Criterion) {
    c.bench_function("signal_set_1000_subscribers", |b| {
        Runtime::scope("bench_fanout_1000", || {
            let n = signal(0_i32);
            let _sinks = install_subscribers(n, 1000);
            let mut i = 0_i32;
            b.iter(|| {
                i = i.wrapping_add(1);
                n.set(black_box(i));
            });
        });
    });
}

/// Stress shape — no fixed DR-009 budget. Documents the linear cost
/// curve so a future regression on the fan-out path is visible.
fn bench_set_10000_subscribers(c: &mut Criterion) {
    c.bench_function("signal_set_10000_subscribers", |b| {
        Runtime::scope("bench_fanout_10000", || {
            let n = signal(0_i32);
            let _sinks = install_subscribers(n, 10_000);
            let mut i = 0_i32;
            b.iter(|| {
                i = i.wrapping_add(1);
                n.set(black_box(i));
            });
        });
    });
}

/// Build a derived chain of length `depth` rooted at `root`. Each
/// link reads its predecessor and adds 1, so propagation has to walk
/// every level.
///
/// Returns the terminal signal. Caller is responsible for keeping the
/// enclosing scope alive — the chain holds no extra references because
/// `Signal<T>` is a `Copy` arena handle.
fn build_derived_chain(
    root: sigil_reactive::Signal<i32>,
    depth: usize,
) -> sigil_reactive::Signal<i32> {
    let mut current = root;
    for _ in 0..depth {
        let prev = current;
        current = derived(move || prev.get().wrapping_add(1));
    }
    current
}

/// Depth-10 derived chain. `iter_batched` so each iteration runs in
/// its own fresh `Runtime::scope`; the timed body builds the chain
/// and writes once, capturing both first-build subscription cost and
/// the propagation through 10 derived links to a terminal effect.
fn bench_derived_chain_depth_10(c: &mut Criterion) {
    c.bench_function("derived_chain_depth_10", |b| {
        b.iter_batched(
            || (),
            |()| {
                Runtime::scope("bench_chain10", || {
                    let root = signal(0_i32);
                    let tail = build_derived_chain(root, 10);
                    let sink = Rc::new(Cell::new(0_i32));
                    let sink_for_effect = Rc::clone(&sink);
                    effect(move || sink_for_effect.set(tail.get()));
                    // Timed: one write that has to propagate 10 levels
                    // and reach the terminal effect.
                    root.set(black_box(7));
                    black_box(sink.get());
                });
            },
            BatchSize::SmallInput,
        );
    });
}

/// Depth-100 derived chain — deep reactive graph territory. Same
/// shape as depth-10 with the chain length raised. Documents how
/// propagation latency scales with chain depth; no DR-009 budget
/// applies, but a >linear blow-up would point at scheduler quadratic
/// behaviour.
fn bench_derived_chain_depth_100(c: &mut Criterion) {
    c.bench_function("derived_chain_depth_100", |b| {
        b.iter_batched(
            || (),
            |()| {
                Runtime::scope("bench_chain100", || {
                    let root = signal(0_i32);
                    let tail = build_derived_chain(root, 100);
                    let sink = Rc::new(Cell::new(0_i32));
                    let sink_for_effect = Rc::clone(&sink);
                    effect(move || sink_for_effect.set(tail.get()));
                    root.set(black_box(7));
                    black_box(sink.get());
                });
            },
            BatchSize::SmallInput,
        );
    });
}

/// Diamond fan-in — one effect reads 100 distinct signals (so its
/// `deps` smallvec ends up 100 entries long). Each iteration writes
/// to *one* of those signals and the effect re-runs, which forces
/// the deps-clear path to walk 100 entries every time.
///
/// Worst case for the per-run "unsubscribe from all old deps" loop in
/// `effect::run_setup`. If that path ever goes quadratic this bench
/// will spike.
fn bench_diamond_fanout_100_signals_one_effect(c: &mut Criterion) {
    c.bench_function("diamond_fanout_100_signals_one_effect", |b| {
        Runtime::scope("bench_diamond_100", || {
            let mut signals = Vec::with_capacity(100);
            for _ in 0..100 {
                signals.push(signal(0_i32));
            }
            let sinks_signal = signals.clone();
            let sink = Rc::new(Cell::new(0_i32));
            let sink_for_effect = Rc::clone(&sink);
            effect(move || {
                let mut acc = 0_i32;
                for s in &sinks_signal {
                    acc = acc.wrapping_add(s.get());
                }
                sink_for_effect.set(acc);
            });
            // Round-robin which signal we write so we exercise the
            // whole subscriber-lookup path, not just the first slot.
            let mut idx = 0_usize;
            let mut counter = 0_i32;
            b.iter(|| {
                counter = counter.wrapping_add(1);
                // `signals` is a Vec<Signal<i32>> built above with 100
                // entries; `idx % 100` keeps us in-bounds. We use
                // `get` rather than direct indexing to stay clear of
                // the `clippy::indexing_slicing` deny rule that the
                // workspace enforces.
                if let Some(s) = signals.get(idx % 100) {
                    s.set(black_box(counter));
                }
                idx = idx.wrapping_add(1);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_set_100_subscribers,
    bench_set_1000_subscribers,
    bench_set_10000_subscribers,
    bench_derived_chain_depth_10,
    bench_derived_chain_depth_100,
    bench_diamond_fanout_100_signals_one_effect,
);
criterion_main!(benches);
