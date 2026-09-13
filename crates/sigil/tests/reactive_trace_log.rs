#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::reactive::{__trace_test_hook, Runtime, effect, effect_named, signal};

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

            effect_named("reader.sum", move || {
                seen_for.set(a.get() + b.get());
            });
            __trace_test_hook::reset_last_line();

            a.set(10);

            let line =
                __trace_test_hook::take_last_line().expect("effect fire should record a line");

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
    use sigil::reactive::batch;

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

            batch(|| {
                a.set(1);
                b.set(2);
            });

            let line = __trace_test_hook::take_last_line()
                .expect("batched drain should record at least one fire");
            assert!(
                line.contains("writes_pending=["),
                "pending field missing: {line:?}"
            );
        });
    });
}
