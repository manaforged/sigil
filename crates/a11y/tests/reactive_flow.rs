//! W5.2 / DR-050 §G10-phase-2 — end-to-end reactive a11y flow.
//!
//! Wires together [`reactive::A11yObserver`],
//! [`reactive::Runtime::install_a11y_publisher`], the kernel
//! `Cx`, and the [`a11y::RecordingBridge`] to prove that a
//! checkbox-style reactive `checked` signal flips the AccessKit tree
//! at end-of-frame.

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns
)]

// `hal` provides the `A11yBridge` trait the recording bridge
// implements; load-bearing through `a11y::RecordingBridge` but
// not named directly in this test.
use hal as _;
// `sigil` is a dev-dep for Phase 1.3.C's `live_regions.rs`; every
// integration test pulls all dev-deps, so alias here too.
use sigil as _;
// W5-A11Y's `AccessKitBridge` backing dep, unused by this file's
// `RecordingBridge`-based flow. `layout` backs RC-focus finding 1's
// `bounds_projection.rs`, also unused here.
use crossbeam_channel as _;
use layout as _;

use std::cell::RefCell;
use std::rc::Rc;

use a11y::{RecordingBridge, snapshot, tree};
use accesskit::Role;
use kernel::{Cx, container};
use reactive::{A11yObserver, Runtime, Signal, signal};

type SharedCx = Rc<RefCell<Cx>>;
type SharedBridge = Rc<RefCell<RecordingBridge>>;

/// Drive the full frame phase cycle so the end-of-frame drain fires.
fn drive_frame() {
    Runtime::begin_effect();
    Runtime::begin_layout();
    Runtime::begin_paint();
    Runtime::end_frame();
}

/// Install a publisher that snapshots `cx` through `a11y::tree`
/// and forwards the result into `bridge`.
fn install_cx_publisher(cx: &SharedCx, bridge: &SharedBridge) {
    let cx_for_pub = Rc::clone(cx);
    let bridge_for_pub = Rc::clone(bridge);
    Runtime::install_a11y_publisher(Box::new(move || {
        let update = tree(&cx_for_pub.borrow(), None, &|_| None);
        let _ = hal::A11yBridge::publish(&mut *bridge_for_pub.borrow_mut(), update);
    }));
}

/// Build a checkbox-style container with the reactive `checked` signal.
fn add_checkbox(cx: &SharedCx, checked: Signal<bool>) {
    let _ = cx.borrow_mut().add(
        container()
            .role(Role::CheckBox)
            .label("Accept terms")
            .checked(checked),
    );
}

/// Snapshot the bridge's most recent published `TreeUpdate` as text.
fn last_snapshot(bridge: &SharedBridge) -> String {
    let b = bridge.borrow();
    let last = b.last_published().expect("expected at least one publish");
    snapshot(last)
}

#[test]
fn checkbox_signal_change_publishes_tree_update() {
    let cx: SharedCx = Rc::new(RefCell::new(Cx::new_root()));
    let bridge: SharedBridge = Rc::new(RefCell::new(RecordingBridge::new()));

    Runtime::scope("checkbox_signal_change_publishes_tree_update", || {
        let checked = signal(false);
        add_checkbox(&cx, checked);
        install_cx_publisher(&cx, &bridge);

        let observer = A11yObserver::new();
        observer.observe_bool(checked);

        // Settle the initial-registration publish.
        drive_frame();
        assert_eq!(
            bridge.borrow().published_count(),
            1,
            "first frame publishes initial tree state"
        );
        let initial_snap = last_snapshot(&bridge);
        assert!(
            initial_snap.contains("Toggled=False"),
            "initial publish shows Toggled=False, got:\n{initial_snap}"
        );

        // Mutate the signal — drives a new publish at end_frame.
        checked.set(true);
        drive_frame();

        assert_eq!(
            bridge.borrow().published_count(),
            2,
            "checked.set(true) produces a second publish"
        );

        let updated_snap = last_snapshot(&bridge);
        assert!(
            updated_snap.contains("Toggled=True"),
            "second publish shows Toggled=True, got:\n{updated_snap}"
        );
        assert!(
            updated_snap.contains("Role=CheckBox"),
            "snapshot preserves the checkbox role, got:\n{updated_snap}"
        );
        assert!(
            updated_snap.contains("Name=\"Accept terms\""),
            "snapshot preserves the accessible name, got:\n{updated_snap}"
        );

        observer.dispose();

        // Uninstall + drop the publisher BEFORE the scope unwinds so
        // its `Rc<RefCell<Cx>>` capture releases its borrow on the
        // shared Cx; otherwise the runtime TLS destructor races the
        // kernel root owner's disposal and aborts the process.
        let publisher = Runtime::take_a11y_publisher();
        drop(publisher);
    });

    // Drop the Cx (and its kernel root owner) only after the
    // reactive runtime scope has fully unwound.
    drop(cx);
    drop(bridge);
}
