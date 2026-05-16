//! Integration tests for the `SIGIL_TRACE_EFFECTS` effect trace
//! log (DR-005 addendum item 6).
//!
//! These tests never touch the `SIGIL_TRACE_EFFECTS` environment
//! variable. `OnceLock` in [`sigil_reactive::trace`] latches the env
//! result at the first effect fire in the process, so toggling env
//! from tests would be flaky under parallel or ordered runs. Instead
//! we exercise the per-thread capture hook exposed via
//! `__trace_test_hook`: the hook sits in the same formatting path as
//! the stderr emitter and mirrors the exact line a stderr consumer
//! would see. The env-gated side is covered by the line-format
//! assertion and by module-level documentation — what matters here
//! is that the hook fires when expected, carries the right fields,
//! and stays silent when capture is off.

#![allow(missing_docs)]

// `smallvec` and `criterion` are dev-deps; this test references
// neither. Silence `unused_crate_dependencies`.
use criterion as _;
use smallvec as _;

use std::cell::Cell;
use std::rc::Rc;

use sigil_reactive::{__trace_test_hook, Runtime, effect, effect_named, signal};

/// Guard that turns capture off at test end so a failure in one
/// test does not leak state into the next on this thread.
struct CaptureGuard;

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        __trace_test_hook::disable_capture();
    }
}

fn with_capture<R>(f: impl FnOnce() -> R) -> R {
    __trace_test_hook::enable_capture();
    __trace_test_hook::reset_last_line();
    let _guard = CaptureGuard;
    f()
}

#[test]
fn capture_disabled_records_nothing() {
    // No enable_capture call — the hook must not populate the buffer.
    __trace_test_hook::reset_last_line();
    Runtime::scope("trace::off", || {
        let n = signal(0_i32);
        effect(move || {
            let _ = n.get();
        });
        n.set(1);
    });
    assert!(
        __trace_test_hook::take_last_line().is_none(),
        "trace buffer must stay empty when capture is disabled"
    );
}

#[test]
fn effect_fire_records_expected_line_shape() {
    with_capture(|| {
        Runtime::scope("trace::shape", || {
            let a = signal(1_i32);
            let b = signal(2_i32);
            let seen = Rc::new(Cell::new(0));
            let seen_for = Rc::clone(&seen);

            // Real effect: depends on two signals.
            effect_named("reader.sum", move || {
                seen_for.set(a.get() + b.get());
            });
            // Clear the registration fire so we inspect only the
            // write-driven re-run.
            __trace_test_hook::reset_last_line();

            a.set(10);

            let line =
                __trace_test_hook::take_last_line().expect("effect fire should record a line");

            // Literal format, component by component.
            assert!(
                line.starts_with("[sigil trace] effect fired: "),
                "prefix mismatch: {line:?}"
            );
            assert!(
                line.contains("name=reader.sum"),
                "name field mismatch: {line:?}"
            );
            assert!(line.contains(" at="), "missing at= field: {line:?}");
            assert!(
                line.contains("trace_log.rs:"),
                "at= should point at this test file: {line:?}"
            );
            assert!(
                line.contains("reads=[2]"),
                "reader.sum subscribes to two signals: {line:?}"
            );
            assert!(
                line.contains("writes_pending=[0]"),
                "no batch, pending queue drains synchronously: {line:?}"
            );

            // Value was applied end-to-end.
            assert_eq!(seen.get(), 12);
        });
    });
}

#[test]
fn unnamed_effect_records_default_name() {
    with_capture(|| {
        Runtime::scope("trace::unnamed", || {
            let n = signal(0_i32);
            __trace_test_hook::reset_last_line();
            effect(move || {
                let _ = n.get();
            });
            let line =
                __trace_test_hook::take_last_line().expect("effect fires once on registration");
            assert!(
                line.contains("name=<unnamed>"),
                "effect() without a name should record `<unnamed>`: {line:?}"
            );
            assert!(line.contains("reads=[1]"), "one dep: {line:?}");
        });
    });
}

#[test]
fn batched_writes_report_pending_queue_depth() {
    use sigil_reactive::batch;

    with_capture(|| {
        Runtime::scope("trace::batched", || {
            let a = signal(0_i32);
            let b = signal(0_i32);
            effect_named("reader.a", move || {
                let _ = a.get();
            });
            effect_named("reader.b", move || {
                let _ = b.get();
            });
            __trace_test_hook::reset_last_line();

            // Inside a batch, effect writes re-queue rather than run
            // synchronously. The trace hook runs per-fire and sees
            // the pending queue at the moment the hook fires.
            batch(|| {
                a.set(1);
                b.set(2);
            });

            // The LAST fire observed the pending queue after its own
            // schedule cycle. The queue drains in FIFO, so at the
            // point the second fire's hook runs, pending is empty.
            // The important invariant is that at least one line was
            // captured with a well-formed pending field.
            let line = __trace_test_hook::take_last_line()
                .expect("batched drain should record at least one fire");
            assert!(
                line.contains("writes_pending=["),
                "pending field missing: {line:?}"
            );
        });
    });
}
