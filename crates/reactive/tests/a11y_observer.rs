//! W5.2 / DR-050 §G10-phase-2 — `A11yObserver` reactive surface tests.
//!
//! These tests exercise the reactive observer in isolation from
//! `sigil-a11y` and `sigil-kernel`: the "publisher" the tests install
//! is just a `Cell<u32>` counter (and a record of the values seen on
//! the signal at publish time). The integration test in
//! `crates/a11y/tests/reactive_flow.rs` exercises the full
//! `tree(cx) → bridge.publish` pipe.

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns
)]

use actions as _;
use criterion as _;
use smallvec as _;
use tokio as _;

use std::cell::Cell;
use std::rc::Rc;

use reactive::{A11yObserver, Runtime, signal};

/// Helper — drives the full frame phase cycle so end-of-frame drain
/// fires.
fn drive_frame() {
    Runtime::begin_effect();
    Runtime::begin_layout();
    Runtime::begin_paint();
    Runtime::end_frame();
}

#[test]
fn observer_fires_on_checked_signal_change() {
    let publish_count = Rc::new(Cell::new(0_u32));
    let seen = Rc::new(Cell::new(false));

    Runtime::scope("observer_fires_on_checked_signal_change", || {
        let checked = signal(false);

        let count = Rc::clone(&publish_count);
        let s = Rc::clone(&seen);
        Runtime::install_a11y_publisher(Box::new(move || {
            count.set(count.get() + 1);
            s.set(checked.peek());
        }));

        let observer = A11yObserver::new();
        observer.observe_bool(checked);

        // Initial effect run fires inside `observe_bool` and marks
        // the runtime dirty. Drive one frame so the initial publish
        // settles, then assert subsequent state-change behaviour.
        drive_frame();
        assert_eq!(
            publish_count.get(),
            1,
            "initial effect registration fires the dirty flag, drains once"
        );

        // Real test: mutate the signal, drive a frame, expect a
        // second publish with the new value visible to the
        // closure-captured `seen` recorder.
        checked.set(true);
        drive_frame();

        assert_eq!(publish_count.get(), 2, "checked.set fires a publish");
        assert!(seen.get(), "publisher saw the updated signal value");

        drop(observer);
    });
}

#[test]
fn observer_skips_untouched_nodes() {
    let publish_count = Rc::new(Cell::new(0_u32));

    Runtime::scope("observer_skips_untouched_nodes", || {
        let checked = signal(false);

        let count = Rc::clone(&publish_count);
        Runtime::install_a11y_publisher(Box::new(move || {
            count.set(count.get() + 1);
        }));

        let observer = A11yObserver::new();
        observer.observe_bool(checked);

        // First frame consumes the initial-registration mark.
        drive_frame();
        assert_eq!(publish_count.get(), 1, "initial registration marks dirty");

        // Subsequent frames without any signal mutation must NOT
        // republish — drain_a11y_updates is a strict no-op when the
        // flag is clean.
        drive_frame();
        drive_frame();
        drive_frame();
        assert_eq!(
            publish_count.get(),
            1,
            "frames without signal changes produce no spurious publishes"
        );

        drop(observer);
    });
}

#[test]
fn drain_is_idempotent() {
    let publish_count = Rc::new(Cell::new(0_u32));

    Runtime::scope("drain_is_idempotent", || {
        let checked = signal(false);

        let count = Rc::clone(&publish_count);
        Runtime::install_a11y_publisher(Box::new(move || {
            count.set(count.get() + 1);
        }));

        let observer = A11yObserver::new();
        observer.observe_bool(checked);

        // Settle initial registration.
        drive_frame();
        let baseline = publish_count.get();

        // Mutate once, drive two frames back-to-back. The second
        // frame must see a clean dirty flag (cleared by the first
        // drain) and not republish.
        checked.set(true);
        drive_frame();
        let after_first = publish_count.get();
        drive_frame();
        let after_second = publish_count.get();

        assert_eq!(
            after_first - baseline,
            1,
            "first drain after the mutation publishes exactly once"
        );
        assert_eq!(
            after_second, after_first,
            "second drain is a no-op — the dirty flag was cleared"
        );

        drop(observer);
    });
}

#[test]
fn drop_deregisters() {
    let publish_count = Rc::new(Cell::new(0_u32));

    Runtime::scope("drop_deregisters", || {
        let checked = signal(false);

        let count = Rc::clone(&publish_count);
        Runtime::install_a11y_publisher(Box::new(move || {
            count.set(count.get() + 1);
        }));

        let observer = A11yObserver::new();
        observer.observe_bool(checked);

        // Settle initial registration.
        drive_frame();
        let baseline = publish_count.get();

        // Dispose the observer — its owner cascades and the
        // tracking effect dies. Mutating the signal afterwards must
        // NOT mark the runtime dirty.
        observer.dispose();

        checked.set(true);
        drive_frame();

        assert_eq!(
            publish_count.get(),
            baseline,
            "no publish fires after the observer's tracking effect is gone"
        );
    });
}

#[test]
fn observer_handles_multiple_signal_types() {
    // Smoke test that observe_f32 / observe_string mark dirty the
    // same way observe_bool does, so the helper API actually covers
    // value_now (slider) and text_value (text input) end-to-end.
    let publish_count = Rc::new(Cell::new(0_u32));

    Runtime::scope("observer_handles_multiple_signal_types", || {
        let value = signal(0.0_f32);
        let text = signal(String::from("hello"));

        let count = Rc::clone(&publish_count);
        Runtime::install_a11y_publisher(Box::new(move || {
            count.set(count.get() + 1);
        }));

        let observer = A11yObserver::new();
        observer.observe_f32(value);
        observer.observe_string(text);

        // Settle initial registration.
        drive_frame();
        let baseline = publish_count.get();

        value.set(0.5);
        drive_frame();
        assert!(
            publish_count.get() > baseline,
            "f32 signal change marks dirty"
        );
        let after_f32 = publish_count.get();

        text.set(String::from("world"));
        drive_frame();
        assert!(
            publish_count.get() > after_f32,
            "String signal change marks dirty"
        );

        drop(observer);
    });
}
