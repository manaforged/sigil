use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::hal::Scheduler;
use crate::reactive::Owner;
use crate::tasks::{AsyncRuntime, TaskWorkers};

#[test]
fn runtime_without_spawn_starts_no_workers() {
    Owner::scope("runtime_without_spawn_starts_no_workers", || {
        let mut runtime = AsyncRuntime::new().map_err(|e| e.to_string());
        let Ok(runtime) = runtime.as_mut() else {
            return assert!(runtime.is_ok());
        };
        runtime.install_current();
        let _ = runtime.pump();
        assert!(!runtime.runtime_started());
    });
}

#[test]
fn fixed_workers_run_spawned_task() {
    Owner::scope("fixed_workers_run_spawned_task", || {
        let workers = TaskWorkers::Fixed(NonZeroUsize::MIN.saturating_add(1));
        let runtime = AsyncRuntime::with_workers(workers);
        let Some(owner) = Owner::current() else {
            return assert!(Owner::current().is_some());
        };
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let _task = runtime.spawn(owner, async move {
            flag.store(true, Ordering::SeqCst);
        });
        assert!(runtime.runtime_started());
        let start = Instant::now();
        while !ran.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ran.load(Ordering::SeqCst));
    });
}

#[test]
fn dropping_the_runtime_does_not_wait_for_blocking_work() {
    let (release, stuck) = std::sync::mpsc::channel::<()>();
    let (dropped, done) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        Owner::scope(
            "dropping_the_runtime_does_not_wait_for_blocking_work",
            || {
                let Ok(runtime) = AsyncRuntime::new() else {
                    return;
                };
                let Some(owner) = Owner::current() else {
                    return;
                };
                let (started, running) = std::sync::mpsc::channel();
                let _task = runtime.spawn(owner, async move {
                    crate::tasks::blocking(move || {
                        let _ = started.send(());
                        let _ = stuck.recv();
                    })
                    .await;
                });
                let _ = running.recv_timeout(Duration::from_secs(10));
                drop(runtime);
                let _ = dropped.send(());
            },
        );
    });

    let outcome = done.recv_timeout(Duration::from_secs(10));
    let _ = release.send(());
    assert!(
        outcome.is_ok(),
        "dropping the runtime waited on a blocking job"
    );
}
