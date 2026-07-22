//! Floating combat text micro-benchmarks.
//!
//! Targets the F6 acceptance gate: 500 simultaneous spawns + per-frame
//! animations under 2 ms per frame on the dev box. The bench measures
//! the simulation step in isolation (no layout, no scene emit), which
//! is the hot path the widget owns.
//!
//! Run with `cargo bench -p sigil-ui --bench floating_text`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::panic
)]

// Workspace deps that the bench target inherits but does not call.
use arboard as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_renderer as _;
use sigil_tasks as _;
use wgpu as _;

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

/// 500 spawns into an empty pool (cold burst).
fn bench_spawn_500(c: &mut Criterion) {
    c.bench_function("fct_spawn_500_cold", |b| {
        Runtime::scope("fct-bench-spawn", || {
            b.iter(|| {
                let state = make_state(2.0);
                for i in 0..500_u32 {
                    #[allow(clippy::cast_precision_loss)]
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

/// Steady-state per-frame tick over 500 live particles.
fn bench_tick_500(c: &mut Criterion) {
    c.bench_function("fct_tick_500_steady", |b| {
        Runtime::scope("fct-bench-tick", || {
            let state = make_state(10.0);
            for i in 0..500_u32 {
                #[allow(clippy::cast_precision_loss)]
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

/// Combined per-frame cost: spawn 8 + tick over a 500-live pool. This
/// is the headline F6 number — "500 simultaneous spawns + animations
/// ≤ 2ms/frame".
fn bench_frame_500(c: &mut Criterion) {
    c.bench_function("fct_frame_8spawn_500live", |b| {
        Runtime::scope("fct-bench-frame", || {
            let state = make_state(10.0);
            for i in 0..500_u32 {
                #[allow(clippy::cast_precision_loss)]
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
                // Spawn a small burst per frame to exercise the slot
                // search; in a real game 1-10 new numbers per frame is
                // typical, 8 is a generous upper bound.
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

/// Pool saturation: tick 1024 live particles (worst-case pool walk).
fn bench_tick_pool_full(c: &mut Criterion) {
    c.bench_function("fct_tick_pool_full", |b| {
        Runtime::scope("fct-bench-pool-full", || {
            let state = make_state(10.0);
            for i in 0..FCT_POOL_CAPACITY {
                #[allow(clippy::cast_precision_loss)]
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
