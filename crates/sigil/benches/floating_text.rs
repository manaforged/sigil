#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::panic
)]

use std::hint::black_box;
use std::rc::Rc;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::widgets::{FCT_POOL_CAPACITY, FctSpawn, FctState};
use sigil_reactive::{Runtime, signal};
use sigil_style::Rgba8;

const RED: Rgba8 = Rgba8 {
    r: 240,
    g: 64,
    b: 64,
    a: 255,
};

fn make_state(duration: f32) -> Rc<FctState> {
    let time = signal(0.0_f32);
    Rc::new(FctState::new(
        duration,
        [0.0, -32.0],
        time,
        FCT_POOL_CAPACITY,
    ))
}

fn bench_spawn_500(c: &mut Criterion) {
    c.bench_function("fct_spawn_500_cold", |b| {
        Runtime::scope("fct-bench-spawn", || {
            b.iter(|| {
                let state = make_state(2.0);
                for i in 0..500_u32 {
                    #[allow(clippy::cast_precision_loss, reason = "pixel/char magnitudes are bounded; sub-unit precision is irrelevant")]
                    let f = i as f32;
                    let payload = FctSpawn {
                        at: [f, f],
                        text: i.to_string(),
                        color: RED,
                    };
                    let _ok = state.spawn(payload, 0.0);
                }
                black_box(state.alive_count());
            });
        });
    });
}

fn bench_tick_500(c: &mut Criterion) {
    c.bench_function("fct_tick_500_steady", |b| {
        Runtime::scope("fct-bench-tick", || {
            let state = make_state(10.0);
            for i in 0..500_u32 {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "pixel/char magnitudes are bounded; sub-unit precision is irrelevant"
                )]
                let f = i as f32;
                let payload = FctSpawn {
                    at: [f, f],
                    text: i.to_string(),
                    color: RED,
                };
                let _ok = state.spawn(payload, 0.0);
            }
            let mut t = 0.0_f32;
            b.iter(|| {
                t += 1.0 / 60.0;
                state.tick(t);
                black_box(state.alive_count());
            });
        });
    });
}

fn bench_frame_500(c: &mut Criterion) {
    c.bench_function("fct_frame_8spawn_500live", |b| {
        Runtime::scope("fct-bench-frame", || {
            let state = make_state(10.0);
            for i in 0..500_u32 {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "pixel/char magnitudes are bounded; sub-unit precision is irrelevant"
                )]
                let f = i as f32;
                let payload = FctSpawn {
                    at: [f, f],
                    text: i.to_string(),
                    color: RED,
                };
                let _ok = state.spawn(payload, 0.0);
            }
            let mut t = 0.0_f32;
            let mut counter: u32 = 0;
            b.iter(|| {
                t += 1.0 / 60.0;
                state.tick(t);
                for _ in 0..8 {
                    counter = counter.wrapping_add(1);
                    let payload = FctSpawn {
                        at: [0.0, 0.0],
                        text: counter.to_string(),
                        color: RED,
                    };
                    let _ok = state.spawn(payload, t);
                }
                black_box(state.alive_count());
            });
        });
    });
}

fn bench_tick_pool_full(c: &mut Criterion) {
    c.bench_function("fct_tick_pool_full", |b| {
        Runtime::scope("fct-bench-pool-full", || {
            let state = make_state(10.0);
            for i in 0..FCT_POOL_CAPACITY {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "pixel/char magnitudes are bounded; sub-unit precision is irrelevant"
                )]
                let f = i as f32;
                let payload = FctSpawn {
                    at: [f, f],
                    text: "x".to_string(),
                    color: RED,
                };
                let _ok = state.spawn(payload, 0.0);
            }
            let mut t = 0.0_f32;
            b.iter(|| {
                t += 1.0 / 60.0;
                state.tick(t);
                black_box(state.alive_count());
            });
        });
    });
}

criterion_group!(
    benches,
    bench_spawn_500,
    bench_tick_500,
    bench_frame_500,
    bench_tick_pool_full,
);
criterion_main!(benches);
