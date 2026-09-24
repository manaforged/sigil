#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::reactive::{Owner, Runtime, batch, effect, signal};

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
