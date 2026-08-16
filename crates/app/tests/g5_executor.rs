//! G5 (DR-048 §5) — production install path end-to-end.
//!
//! Drives the same wiring `launch::run` uses (`AsyncRuntime` →
//! `install_g5_executor` → `pump_completions_into_reactive` →
//! `Runtime::begin_effect`) against a fixture that does not boot
//! winit or wgpu. The tests confirm that a future spawned via
//! `Context::spawn` against the real Tokio multi-thread runtime
//! actually completes, posts its callback through the reactive
//! bridge, and lands inside the next frame's Effect phase.
//!
//! These complement `crates/reactive/tests/task.rs` (TestExecutor
//! synchronous fixture); the divergence here is that the production
//! bridge uses an MPSC + a separate pump step rather than an inline
//! `Mutex<VecDeque>` drain.

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::doc_markdown
)]

// Workspace transitive — sigil-app's full dep tree is pulled into
// every integration test's compilation unit; alias the ones this test
// doesn't reference directly so `-D unused-crate-dependencies` stays
// silent (same pattern as `scale_handling.rs`).
use accesskit as _;
use crossbeam_channel as _;
use muda as _;
use notify as _;
use raw_window_handle as _;
use rfd as _;
use sigil_a11y as _;
use sigil_animation_runtime as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_renderer as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use toml as _;
use tracing as _;
use tray_icon as _;
#[cfg(windows)]
use windows_core as _;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sigil_app::install_g5_executor;
use sigil_reactive::{AsyncContext, Entity, Runtime, Task};
use sigil_tasks::AsyncRuntime;

/// Entity payload that carries a shared atomic counter so the worker
/// thread can prove it observed the entity through `acx.update`.
struct Counter {
    n: Arc<AtomicUsize>,
}

impl Counter {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let n = Arc::new(AtomicUsize::new(0));
        (Self { n: Arc::clone(&n) }, n)
    }
}

/// Busy-wait until `flag >= target` or the deadline elapses. Returns
/// whether the threshold was reached.
fn wait_for_atomic(flag: &AtomicUsize, target: usize, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if flag.load(Ordering::SeqCst) >= target {
            return true;
        }
        thread::sleep(Duration::from_millis(1));
    }
    flag.load(Ordering::SeqCst) >= target
}

/// Walk one production-shaped frame: pump the bridge, then run the
/// reactive frame phases. Mirrors `launch::frame_once` in shape, but
/// without the winit/wgpu side.
fn drive_one_frame(async_rt: &mut AsyncRuntime) -> usize {
    let moved = async_rt.pump_completions_into_reactive();
    Runtime::begin_effect();
    Runtime::begin_layout();
    Runtime::begin_paint();
    Runtime::end_frame();
    moved
}

#[test]
fn spawn_completion_arrives_at_next_frame_begin_effect() {
    Runtime::scope("g5_single_completion", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        install_g5_executor(&async_rt);

        let (counter, n) = Counter::new();
        let entity = Entity::<Counter>::new("counter", |_| counter);
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(|acx: AsyncContext<Counter>| async move {
                acx.update(|this, _cx| {
                    this.n.fetch_add(1, Ordering::SeqCst);
                })
                .await
                .expect("entity still alive when callback posted");
            })
        });

        // The worker has to (a) run the future, (b) post the closure
        // through the bridge MPSC. Once the MPSC has the entry, we
        // pump and drive a frame; the entity mutation should land.
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut frames_driven = 0_usize;
        while Instant::now() < deadline {
            let moved = drive_one_frame(&mut async_rt);
            frames_driven += 1;
            if moved > 0 && n.load(Ordering::SeqCst) >= 1 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            n.load(Ordering::SeqCst) >= 1,
            "future completion landed within timeout (after {frames_driven} frame(s))",
        );

        // Detach so the Task drop doesn't try to abort a finished task
        // (no-op, but keeps the test's intent explicit).
        task.detach();
    });
}

#[test]
fn multi_task_concurrent_completion() {
    Runtime::scope("g5_multi_completion", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        install_g5_executor(&async_rt);

        let (counter, n) = Counter::new();
        let entity = Entity::<Counter>::new("multi", |_| counter);

        // Spawn five tasks; each increments the same counter via
        // `acx.update`. The multi-thread tokio runtime fans them out
        // across worker threads.
        let mut tasks: Vec<Task<()>> = Vec::with_capacity(5);
        for _ in 0..5_u32 {
            let t = entity.update(|_, cx| {
                cx.spawn(|acx: AsyncContext<Counter>| async move {
                    acx.update(|this, _cx| {
                        this.n.fetch_add(1, Ordering::SeqCst);
                    })
                    .await
                    .expect("entity alive on completion");
                })
            });
            tasks.push(t);
        }

        // Drive frames until five mutations land or we time out.
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut total_moved = 0_usize;
        while Instant::now() < deadline && n.load(Ordering::SeqCst) < 5 {
            total_moved += drive_one_frame(&mut async_rt);
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            n.load(Ordering::SeqCst),
            5,
            "every spawn's completion landed exactly once (pumped {total_moved} callback(s))",
        );

        for t in tasks {
            t.detach();
        }
    });
}

/// Negative-path test — `pump_completions_into_reactive` is a no-op
/// when no worker has posted anything, and the frame loop stays
/// quiescent.
#[test]
fn pump_returns_zero_when_nothing_pending() {
    Runtime::scope("g5_quiescent", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        install_g5_executor(&async_rt);

        // No spawns: bridge is empty.
        let moved = drive_one_frame(&mut async_rt);
        assert_eq!(moved, 0, "quiescent frame moves zero completions");

        // Drive a second frame to confirm no leftover state pollution.
        let moved_two = drive_one_frame(&mut async_rt);
        assert_eq!(moved_two, 0, "second quiescent frame also moves zero");
    });
}

/// Wait for the bridge waker to fire after a worker posts. Confirms the
/// `ReactiveBridge`-supplied waker actually runs on the producer side.
#[test]
fn bridge_waker_fires_per_post() {
    Runtime::scope("g5_waker", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        let wakes = Arc::new(AtomicUsize::new(0));
        let wakes_for_waker = Arc::clone(&wakes);
        async_rt.set_bridge_waker(move || {
            wakes_for_waker.fetch_add(1, Ordering::SeqCst);
        });
        install_g5_executor(&async_rt);

        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("waker_probe", |_| counter);
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(|acx: AsyncContext<Counter>| async move {
                acx.update(|_this, _cx| {}).await.expect("alive");
            })
        });

        assert!(
            wait_for_atomic(&wakes, 1, Duration::from_secs(2)),
            "bridge waker fired at least once for the worker's post",
        );

        // Pump + drive the frame so the test's Drop ordering doesn't
        // hit a still-pending callback.
        drive_one_frame(&mut async_rt);
        task.detach();
    });
}
