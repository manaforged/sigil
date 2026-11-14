#![allow(clippy::expect_used)]

use criterion as _;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::reactive::Owner;
use crate::tasks::{AsyncRuntime, MessageReceiver, Status, listen, task};

pub(super) const WAIT: Duration = Duration::from_secs(5);

pub(super) fn spin_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    cond()
}

fn runtime() -> AsyncRuntime {
    let runtime = AsyncRuntime::new().expect("build tokio runtime");
    runtime.install_current();
    runtime
}

fn drain(rx: impl MessageReceiver<u32>) -> (Vec<u32>, u32) {
    let mut runtime = runtime();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let closes = Rc::new(Cell::new(0_u32));
    let (record, count) = (Rc::clone(&seen), Rc::clone(&closes));
    listen(rx, move |n| record.borrow_mut().push(n)).closed(move || count.set(count.get() + 1));
    assert!(
        spin_until(WAIT, || {
            let _ = runtime.pump();
            closes.get() > 0
        }),
        "the close hook never ran"
    );
    let _ = runtime.pump();
    (seen.take(), closes.get())
}

#[test]
fn listen_delivers_in_order_then_runs_closed_once() {
    Owner::scope("listen-tokio", || {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for n in 1..=3 {
            tx.send(n).expect("receiver alive");
        }
        drop(tx);
        assert_eq!(drain(rx), (vec![1, 2, 3], 1));
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn listen_reads_a_std_receiver() {
    Owner::scope("listen-std", || {
        let (tx, rx) = std::sync::mpsc::channel();
        for n in 1..=3 {
            tx.send(n).expect("receiver alive");
        }
        drop(tx);
        assert_eq!(drain(rx), (vec![1, 2, 3], 1));
    });
}

#[test]
fn closed_set_after_the_channel_closed_still_runs_once() {
    Owner::scope("listen-late-close", || {
        let mut runtime = runtime();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<u32>();
        drop(tx);
        let listener = listen(rx, |_| {});
        assert!(
            spin_until(WAIT, || runtime.pump() > 0),
            "the end of the channel never reached the UI thread"
        );
        let closes = Rc::new(Cell::new(0_u32));
        let count = Rc::clone(&closes);
        listener.closed(move || count.set(count.get() + 1));
        assert_eq!(closes.get(), 1, "a close set late was lost");
        let _ = runtime.pump();
        assert_eq!(closes.get(), 1, "the close ran twice");
    });
}

#[test]
fn a_task_waits_for_run_and_autostart_runs_it_at_creation() {
    Owner::scope("task-autostart", || {
        let mut runtime = runtime();
        let lazy = task(|| async { Ok::<u32, String>(1) });
        let eager = task(|| async { Ok::<u32, String>(2) }).autostart();
        assert!(lazy.status().is_idle(), "task started before run");
        assert!(eager.status().is_pending(), "autostart did not start");
        assert!(
            spin_until(WAIT, || {
                let _ = runtime.pump();
                eager.status().is_ok()
            }),
            "the autostarted task never finished"
        );
        assert!(matches!(eager.status(), Status::Ok(2)));
        assert!(lazy.status().is_idle(), "task ran without run");
    });
}
