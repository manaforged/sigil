#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil_reactive::{Runtime, derived, effect, signal};

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
                    root.set(black_box(7));
                    black_box(sink.get());
                });
            },
            BatchSize::SmallInput,
        );
    });
}

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
            let mut idx = 0_usize;
            let mut counter = 0_i32;
            b.iter(|| {
                counter = counter.wrapping_add(1);
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
