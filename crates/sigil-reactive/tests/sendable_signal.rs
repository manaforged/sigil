//! Tests for `SendableSignal` runtime-identity scoping.
//!
//! `SendableSignal` is the `Send + Sync` identity of a main-thread
//! `Signal` — it exists so `sigil-async::spawn_to_signal` can carry a
//! signal's arena identity into a background task and post results
//! back through the main-thread dispatcher. Re-hydrating on the
//! wrong thread would either look disposed (best case) or silently
//! alias into an unrelated signal (worst case), so every
//! `SendableSignal` embeds the id of the runtime it was minted in
//! and checks it on re-hydration.
//!
//! These tests cover both paths: same-runtime re-hydration works,
//! cross-runtime re-hydration panics with an actionable message.

#![allow(missing_docs)]

// Silence `unused_crate_dependencies` — criterion is a dev-dep for
// benches, smallvec a regular dep; neither is referenced here.
use criterion as _;
use smallvec as _;

use sigil_reactive::{Runtime, signal};

#[test]
fn sendable_signal_rehydration_on_correct_runtime_works() {
    // Happy path: mint a SendableSignal on this runtime, convert it
    // back on the same runtime, and read/write the rehydrated handle
    // as normal. This mirrors what `sigil-async::spawn_to_signal` does
    // when the dispatcher drains a background result on the main
    // thread.
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

    // Mint a SendableSignal on this thread's runtime.
    let (tx, rx) = mpsc::channel();
    Runtime::scope("sendable_wrong_runtime::origin", || {
        let sig = signal(123_i32);
        // `SendableSignal<T>` is `Send` by design — that is the whole
        // point of the wrapper. Ship it to a fresh thread that has
        // its own reactive runtime with its own id.
        tx.send(sig.into_sendable()).expect("send to worker");
    });

    // On a different thread, establish a fresh reactive runtime and
    // try to rehydrate. This must panic — the stored runtime_id
    // does not match the new thread's runtime_id.
    let join = thread::spawn(move || {
        let sendable = rx.recv().expect("recv from main");
        Runtime::scope("sendable_wrong_runtime::worker", || {
            // `into_signal` is where the identity check lives. The
            // boundary check fails fast here, not on a later
            // get/set where a silent alias could do damage.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = sendable.into_signal();
            }));
            result.expect_err("rehydration on wrong runtime must panic")
        })
    });

    let payload = join
        .join()
        .expect("worker thread panicked only under catch_unwind");
    // Pull the panic message out of the catch_unwind payload and
    // assert it names the contract it enforces.
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
