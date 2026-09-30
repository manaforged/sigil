#![allow(clippy::panic, unused_crate_dependencies)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use sigil::reactive::{Owner, Runtime};
use sigil::tasks::{AsyncRuntime, Status, task};
use tokio::sync::Notify;

fn spin_until(mut cond: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    cond()
}

fn runtime() -> (AsyncRuntime, Arc<AtomicBool>) {
    let mut runtime = AsyncRuntime::new().expect("build tokio runtime");
    let woke = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&woke);
    runtime.set_waker(move || flag.store(true, Ordering::Release));
    runtime.install_current();
    (runtime, woke)
}

fn wait_for_result(woke: &AtomicBool) {
    assert!(
        spin_until(|| woke.swap(false, Ordering::AcqRel)),
        "no task result within 2 s"
    );
}

#[test]
fn a_task_is_idle_until_run_and_then_reports_its_error() {
    Runtime::scope("task-error", || {
        let (mut runtime, woke) = runtime();
        let load = task(|| async { Err::<u32, String>("offline".into()) });
        assert!(matches!(load.status(), Status::Idle));
        load.run();
        assert!(load.status().is_pending());
        wait_for_result(&woke);
        runtime.pump();
        assert!(matches!(load.status(), Status::Err(error) if error == "offline"));
    });
}

#[test]
fn a_rerun_drops_the_result_of_the_run_it_replaced() {
    Runtime::scope("task-rerun", || {
        let (mut runtime, woke) = runtime();
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(Notify::new());
        let load = task({
            let (calls, gate) = (Arc::clone(&calls), Arc::clone(&gate));
            move || {
                let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
                let gate = Arc::clone(&gate);
                async move {
                    if call > 1 {
                        gate.notified().await;
                    }
                    Ok::<usize, String>(call)
                }
            }
        });
        load.run();
        wait_for_result(&woke);
        load.run();
        runtime.pump();
        assert!(
            load.status().is_pending(),
            "the first run's result replaced the second run's Pending"
        );
        gate.notify_one();
        wait_for_result(&woke);
        runtime.pump();
        assert!(matches!(load.status(), Status::Ok(2)));
    });
}

struct Dropped(Arc<AtomicBool>);

impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[test]
fn disposing_the_owner_aborts_a_pending_run() {
    Runtime::scope("task-owner", || {
        let (_runtime, _woke) = runtime();
        let dropped = Arc::new(AtomicBool::new(false));
        let owner = Owner::new("component", None);
        let load = owner.with(|| {
            let dropped = Arc::clone(&dropped);
            task(move || {
                let guard = Dropped(Arc::clone(&dropped));
                async move {
                    let _guard = guard;
                    std::future::pending::<Result<(), String>>().await
                }
            })
        });
        load.run();
        owner.dispose();
        assert!(
            spin_until(|| dropped.load(Ordering::Acquire)),
            "the pending run outlived its owner"
        );
    });
}
