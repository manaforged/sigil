use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use super::listen;
use super::sealed::Sealed;
use crate::tasks::tests::{WAIT, spin_until};

fn one_blocking_thread() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .expect("build tokio runtime")
}

#[test]
fn std_receiver_delivers_in_order_then_reports_close() {
    let runtime = one_blocking_thread();
    let (tx, mut rx) = std::sync::mpsc::channel();
    let sender = std::thread::spawn(move || {
        for n in 1..=3_u32 {
            tx.send(n).expect("receiver alive");
        }
    });
    let received = runtime.block_on(async {
        let mut received = Vec::new();
        while let Some(n) = Sealed::recv(&mut rx).await {
            received.push(n);
        }
        received
    });
    sender.join().expect("sender thread");
    assert_eq!(received, vec![1, 2, 3]);
    assert_eq!(Sealed::try_recv(&mut rx), None);
}

#[test]
fn std_receiver_is_released_after_its_reader_is_dropped() {
    let runtime = one_blocking_thread();
    let (tx, mut rx) = std::sync::mpsc::channel::<u32>();
    let waited = runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(10), Sealed::recv(&mut rx)).await
    });
    assert!(waited.is_err(), "the reader got a message nobody sent");
    let freed = runtime
        .block_on(async { tokio::time::timeout(WAIT, tokio::task::spawn_blocking(|| ())).await });
    assert!(
        matches!(freed, Ok(Ok(()))),
        "the dropped reader kept the only blocking thread parked"
    );
    assert!(
        spin_until(WAIT, || tx.send(1).is_err()),
        "the dropped reader kept the receiver alive"
    );
}

#[test]
fn closed_runs_now_when_listen_has_no_scope() {
    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel::<u32>();
    let closes = Rc::new(Cell::new(0_u32));
    let count = Rc::clone(&closes);
    listen(rx, |_| {}).closed(move || count.set(count.get() + 1));
    assert_eq!(
        closes.get(),
        1,
        "the close hook waited on a listener that never ran"
    );
}
