use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::hal::Scheduler;
use crate::reactive::Runtime;
use crate::tasks::{AsyncRuntime, OwnerHandle, TaskWorkers};

#[test]
fn runtime_without_spawn_starts_no_workers() {
    Runtime::scope("runtime_without_spawn_starts_no_workers", || {
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
    Runtime::scope("fixed_workers_run_spawned_task", || {
        let workers = TaskWorkers::Fixed(NonZeroUsize::MIN.saturating_add(1));
        let runtime = AsyncRuntime::with_workers(workers).map_err(|e| e.to_string());
        let Ok(runtime) = runtime.as_ref() else {
            return assert!(runtime.is_ok());
        };
        let Some(owner) = OwnerHandle::current() else {
            return assert!(OwnerHandle::current().is_some());
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
