#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::doc_markdown
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sigil_app::install_g5_executor;
use sigil_reactive::{AsyncContext, Entity, Owner, Runtime, Task, signal};
use sigil_tasks::{AsyncRuntime, signal_writer};

struct Counter {
    n: Arc<AtomicUsize>,
}

impl Counter {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let n = Arc::new(AtomicUsize::new(0));
        (Self { n: Arc::clone(&n) }, n)
    }
}

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

fn drive_one_frame(async_rt: &mut AsyncRuntime) -> usize {
    let moved = async_rt.pump();
    Runtime::begin_effect();
    Runtime::begin_layout();
    Runtime::begin_paint();
    Runtime::end_frame();
    moved
}

#[test]
fn signal_writer_uses_the_production_frame_pump() {
    Runtime::scope("signal_writer_production_pump", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        let wakes = Arc::new(AtomicUsize::new(0));
        let wakes_for_waker = Arc::clone(&wakes);
        async_rt.set_waker(move || {
            wakes_for_waker.fetch_add(1, Ordering::SeqCst);
        });
        async_rt.install_current();
        let values = signal(Vec::<u32>::new());
        let writer = signal_writer(values);

        thread::spawn(move || writer.set(vec![5, 8]).expect("queue signal write"))
            .join()
            .expect("writer thread");

        assert!(values.peek().is_empty(), "write waits for a frame");
        assert_eq!(wakes.load(Ordering::SeqCst), 1, "write wakes the app");
        assert_eq!(drive_one_frame(&mut async_rt), 1);
        assert_eq!(values.peek(), vec![5, 8]);
    });
}

#[test]
fn production_pump_drops_writes_for_disposed_owners() {
    Runtime::scope("signal_writer_disposed_owner", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        async_rt.install_current();
        let parent = Owner::current().expect("root owner");
        let child = Owner::new("signal_writer_child", Some(parent));
        let writer = child.with(|| signal_writer(signal(0_u32)));

        writer.set(1).expect("queue signal write");
        child.dispose();

        assert_eq!(drive_one_frame(&mut async_rt), 0);
    });
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

#[test]
fn pump_returns_zero_when_nothing_pending() {
    Runtime::scope("g5_quiescent", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        install_g5_executor(&async_rt);

        let moved = drive_one_frame(&mut async_rt);
        assert_eq!(moved, 0, "quiescent frame moves zero completions");

        let moved_two = drive_one_frame(&mut async_rt);
        assert_eq!(moved_two, 0, "second quiescent frame also moves zero");
    });
}

#[test]
fn shared_waker_fires_for_bridge_post() {
    Runtime::scope("g5_waker", || {
        let mut async_rt = AsyncRuntime::new().expect("AsyncRuntime::new");
        let wakes = Arc::new(AtomicUsize::new(0));
        let wakes_for_waker = Arc::clone(&wakes);
        async_rt.set_waker(move || {
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

        drive_one_frame(&mut async_rt);
        task.detach();
    });
}
