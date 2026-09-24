#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::reactive::{Runtime, batch, effect, signal};

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

fn bench_sustained_writes_fanout_1k_subscribers_10_frames(c: &mut Criterion) {
    c.bench_function("sustained_writes_fanout_1k_subscribers_10_frames", |b| {
        Runtime::scope("bench_sustained_fanout", || {
            let n = signal(0_i32);
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
