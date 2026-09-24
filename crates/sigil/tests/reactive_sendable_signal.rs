#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::reactive::{Runtime, signal};

#[test]
fn sendable_signal_rehydration_on_correct_runtime_works() {
    Runtime::scope("sendable_same_runtime", || {
        let sig = signal(7_i32);
        let sendable = sig.into_sendable();
        let rehydrated = sendable.into_signal();
        assert_eq!(rehydrated.get(), 7);
        rehydrated.set(42);
        assert_eq!(
            sig.get(),
            42,
            "writes through the rehydrated handle \
                                   hit the same arena slot as the original"
        );
    });
}

#[test]
fn sendable_signal_rehydration_on_wrong_runtime_panics() {
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    Runtime::scope("sendable_wrong_runtime::origin", || {
        let sig = signal(123_i32);
        tx.send(sig.into_sendable()).expect("send to worker");
    });

    let join = thread::spawn(move || {
        let sendable = rx.recv().expect("recv from main");
        Runtime::scope("sendable_wrong_runtime::worker", || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = sendable.into_signal();
            }));
            result.expect_err("rehydration on wrong runtime must panic")
        })
    });

    let payload = join
        .join()
        .expect("worker thread panicked only under catch_unwind");
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .expect("panic payload is a string");
    assert!(
        msg.contains("SendableSignal rehydrated on wrong runtime"),
        "panic message should name the contract; got: {msg}",
    );
    assert!(
        msg.contains("signals do not cross runtime boundaries"),
        "panic message should explain the invariant; got: {msg}",
    );
}
