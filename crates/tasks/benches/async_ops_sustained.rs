#![allow(unreachable_pub)]

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use criterion::Criterion;
use sigil_hal::Scheduler;
use sigil_reactive::{Runtime, signal};
use sigil_tasks::{AsyncRuntime, OwnerHandle, dispatch_to_owner, spawn_to_signal};

use super::spin_until;

pub fn bench_sustained_cross_thread_writes_1k_per_sec(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_cross_thread_writes_1k_per_sec");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(12));
    group.warm_up_time(Duration::from_secs(1));
    group.bench_function("frame_drain_under_1khz_writes", |b| {
        b.iter_custom(|iters| {
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

fn run_frame_loop(scheduler: &mut AsyncRuntime, total_frames: u32) -> (Duration, Duration) {
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
        let now = Instant::now();
        if now < next_frame_at {
            #[allow(
                clippy::disallowed_methods,
                reason = "bench paces to real wall-clock frame times by design"
            )]
            std::thread::sleep(next_frame_at - now);
        }
    }
    (sum, max)
}

fn run_one_sustained_iteration() -> Duration {
    let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");
    let handle = OwnerHandle::current().expect("inside a reactive scope");
    let dispatcher = scheduler.dispatcher();
    let tick = signal(0_u64);
    let tick_sendable = tick.into_sendable();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_writer = Arc::clone(&stop);
    let writer_started = Instant::now();
    scheduler.spawn(handle, async move {
        let mut counter = 0_u64;
        let period = Duration::from_millis(1);
        let mut next_at = writer_started + period;
        while !stop_writer.load(Ordering::Relaxed) {
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

    let total_frames = 60_u32;
    let (sum, max) = run_frame_loop(&mut scheduler, total_frames);
    stop.store(true, Ordering::Relaxed);
    let _ = scheduler.drain();

    assert!(
        max <= Duration::from_millis(16),
        "frame-drain max {max:?} exceeded 16 ms budget under 1 kHz writes"
    );
    sum
}

pub fn bench_spawn_to_signal_smoke_latency(c: &mut Criterion) {
    c.bench_function("spawn_to_signal_smoke_latency", |b| {
        Runtime::scope("bench_spawn_to_signal", || {
            let mut scheduler = AsyncRuntime::new().expect("build tokio runtime");
            let handle = OwnerHandle::current().expect("inside a reactive scope");
            b.iter(|| {
                let sig = spawn_to_signal::<u32, &'static str, _>(&scheduler, handle, async {
                    Ok(42_u32)
                });
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
