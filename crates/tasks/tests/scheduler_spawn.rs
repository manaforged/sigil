#![allow(clippy::panic, missing_docs, unused_crate_dependencies)]

use std::time::Duration;

use sigil_hal::Scheduler;
use sigil_reactive::{Runtime, signal};
use sigil_tasks::{AsyncRuntime, Status, signal_writer, spawn};

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
