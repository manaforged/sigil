#![allow(clippy::panic, unused_crate_dependencies)]

use std::time::Duration;

use sigil::hal::Scheduler;
use sigil::reactive::{Runtime, signal};
use sigil::tasks::{AsyncRuntime, OwnerHandle, Status, signal_writer, spawn};

fn spin_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    cond()
}

#[test]
fn spawn_writes_ok_on_drain() {
    Runtime::scope("spawn_writes_ok_on_drain", || {
        let mut runtime = AsyncRuntime::new().expect("build tokio runtime");
        runtime.install_current();
        let sig = spawn::<u32, &'static str, _>(async { Ok(7_u32) });
        assert!(sig.peek().is_pending());
        assert!(
            spin_until(Duration::from_secs(2), || {
                let _ = runtime.drain();
                sig.peek().is_ok()
            }),
            "spawn completion should write Status::Ok"
        );
        assert!(matches!(sig.peek(), Status::Ok(7)));
    });
}

#[test]
fn a_panicked_task_reports_panicked_instead_of_pending_forever() {
    Runtime::scope("panicked task resolves", || {
        let mut runtime = AsyncRuntime::new().expect("build tokio runtime");
        runtime.install_current();
        let sig = spawn::<u32, &'static str, _>(async { panic!("dispatch boom") });
        assert!(sig.peek().is_pending());
        assert!(
            spin_until(Duration::from_secs(2), || {
                let _ = runtime.drain();
                sig.peek().is_panicked()
            }),
            "a panicked task must resolve the signal instead of pending forever"
        );
        match sig.peek() {
            Status::Panicked(message) => assert!(message.contains("dispatch boom")),
            _ => panic!("expected Status::Panicked"),
        }
    });
}

#[test]
fn signal_writer_streams_updates_through_drain() {
    Runtime::scope("signal_writer_streams_updates_through_drain", || {
        let mut runtime = AsyncRuntime::new().expect("build tokio runtime");
        runtime.install_current();
        let values = signal(Vec::<u32>::new());
        let writer = signal_writer(values);

        std::thread::spawn(move || {
            writer
                .update(|items| items.extend([2, 3]))
                .expect("queue signal update");
            writer.set(vec![5, 8]).expect("queue signal replacement");
        })
        .join()
        .expect("writer thread");

        assert!(
            values.peek().is_empty(),
            "writes wait for main-thread drain"
        );
        assert_eq!(runtime.drain(), 2);
        assert_eq!(values.peek(), vec![5, 8]);
    });
}

#[test]
fn ui_tasks_share_the_host_runtime_and_leave_it_running() {
    let host = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("host runtime");
    let _entered = host.enter();
    Runtime::scope("host runtime", || {
        let mut runtime = AsyncRuntime::new().expect("UI runtime");
        runtime.install_current();
        drop(AsyncRuntime::new().expect("uninstalled UI runtime"));
        let result = spawn::<_, String, _>(async { Ok(tokio::runtime::Handle::current().id()) });
        assert!(spin_until(Duration::from_secs(2), || {
            runtime.drain();
            result.peek().is_ok()
        }));
        assert!(matches!(result.peek(), Status::Ok(id) if id == host.handle().id()));

        let (finish, waiting) = tokio::sync::oneshot::channel();
        let backend = host.spawn(async move { waiting.await.expect("backend shutdown signal") });
        let ui_task = runtime.spawn(
            OwnerHandle::current().expect("UI owner"),
            std::future::pending(),
        );
        drop(runtime);
        assert!(std::panic::catch_unwind(sigil::tasks::dispatcher).is_err());
        assert!(spin_until(Duration::from_secs(2), || ui_task.is_finished()));
        let cancelled = host.block_on(ui_task);
        assert!(cancelled.expect_err("UI task was cancelled").is_cancelled());
        finish
            .send("backend drained")
            .expect("backend survives UI teardown");
        assert_eq!(
            host.block_on(backend).expect("backend task"),
            "backend drained"
        );
    });
}

#[test]
fn a_current_thread_host_does_not_stall_ui_tasks() {
    let host = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread host");
    let _entered = host.enter();
    Runtime::scope("current-thread host", || {
        let mut runtime = AsyncRuntime::new().expect("UI runtime");
        runtime.install_current();
        let result = spawn::<_, String, _>(async { Ok(tokio::runtime::Handle::current().id()) });
        assert!(spin_until(Duration::from_secs(2), || {
            runtime.drain();
            result.peek().is_ok()
        }));
        assert!(matches!(result.peek(), Status::Ok(id) if id != host.handle().id()));
    });
}
