//! Micro-benchmarks for the sigil-reactive hot paths.
//!
//! These establish a perf baseline for the six operations that
//! matter under real load: signal read, signal write (no subscribers,
//! one subscriber, many subscribers), effect registration, batched
//! coalesced writes, and deep owner-tree disposal.
//!
//! Run with `cargo bench -p sigil-reactive`. Criterion writes
//! `target/criterion/` HTML reports.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

// `smallvec` is a regular dep of `sigil-reactive` (for subscriber
// storage). Benches don't reference it directly; the alias silences
// `unused_crate_dependencies` which flows from regular deps into
// bin/test/bench targets.
use smallvec as _;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_reactive::{Owner, Runtime, batch, effect, signal};

/// Reading a signal: arena lookup, type check, clone, subscribe.
/// No current effect = no subscription path, so this measures the
/// bare read cost.
fn bench_signal_get(c: &mut Criterion) {
    c.bench_function("signal_get_no_effect", |b| {
        Runtime::scope("bench_get", || {
            let n = signal(42_i32);
            b.iter(|| {
                black_box(n.get());
            });
        });
    });
}

/// Writing a signal with no subscribers: slot swap + drop + empty
/// subscriber set clone.
fn bench_signal_set_no_subs(c: &mut Criterion) {
    c.bench_function("signal_set_no_subscribers", |b| {
        Runtime::scope("bench_set_nosub", || {
            let n = signal(0_i32);
            let mut i = 0_i32;
            b.iter(|| {
                i = i.wrapping_add(1);
                n.set(black_box(i));
            });
        });
    });
}

/// Writing a signal with one subscribed effect: slot swap +
/// subscriber clone + schedule + run effect (1 cycle + 4 `with_rt`
/// calls + empty closure).
fn bench_signal_set_one_sub(c: &mut Criterion) {
    c.bench_function("signal_set_one_subscriber", |b| {
        Runtime::scope("bench_set_1sub", || {
            let n = signal(0_i32);
            let sink = Rc::new(Cell::new(0_i32));
            {
                let sink = Rc::clone(&sink);
                effect(move || sink.set(n.get()));
            }
            let mut i = 0_i32;
            b.iter(|| {
                i = i.wrapping_add(1);
                n.set(black_box(i));
            });
        });
    });
}

/// Writing a signal with 10 subscribed effects: subscriber-set
/// clone cost scales here.
fn bench_signal_set_ten_subs(c: &mut Criterion) {
    c.bench_function("signal_set_ten_subscribers", |b| {
        Runtime::scope("bench_set_10sub", || {
            let n = signal(0_i32);
            let sinks: Vec<_> = (0..10).map(|_| Rc::new(Cell::new(0_i32))).collect();
            for sink in &sinks {
                let sink = Rc::clone(sink);
                effect(move || sink.set(n.get()));
            }
            let mut i = 0_i32;
            b.iter(|| {
                i = i.wrapping_add(1);
                n.set(black_box(i));
            });
        });
    });
}

/// Full register+dispose cycle for an effect: create owner, register
/// effect, first run, dispose parent scope.
fn bench_effect_register_and_drop(c: &mut Criterion) {
    c.bench_function("effect_register_and_drop", |b| {
        b.iter(|| {
            Runtime::scope("bench_lifecycle", || {
                let n = signal(0_i32);
                effect(move || {
                    black_box(n.get());
                });
            });
        });
    });
}

/// Batch 100 writes to one signal with one subscriber: drain should
/// run the effect exactly once.
fn bench_batch_coalesce_100(c: &mut Criterion) {
    c.bench_function("batch_coalesce_100_writes", |b| {
        Runtime::scope("bench_batch", || {
            let n = signal(0_i32);
            let sink = Rc::new(Cell::new(0_i32));
            {
                let sink = Rc::clone(&sink);
                effect(move || sink.set(n.get()));
            }
            b.iter(|| {
                batch(|| {
                    for i in 0..100 {
                        n.set(black_box(i));
                    }
                });
            });
        });
    });
}

/// Deep owner tree disposal: 100 nested owners, dispose root.
/// Measures recursive disposal overhead.
fn bench_deep_owner_disposal(c: &mut Criterion) {
    c.bench_function("deep_owner_disposal_100", |b| {
        b.iter(|| {
            Runtime::scope("bench_deep", || {
                let root = Owner::new("root", Owner::current());
                let mut current = root;
                current.with(|| {
                    for i in 0..100_i32 {
                        let child = Owner::new("child", Owner::current());
                        current = child;
                        black_box(i);
                    }
                });
                root.dispose();
            });
        });
    });
}

criterion_group!(
    benches,
    bench_signal_get,
    bench_signal_set_no_subs,
    bench_signal_set_one_sub,
    bench_signal_set_ten_subs,
    bench_effect_register_and_drop,
    bench_batch_coalesce_100,
    bench_deep_owner_disposal,
);
criterion_main!(benches);
