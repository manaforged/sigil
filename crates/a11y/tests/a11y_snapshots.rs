//! DR-050 phase-1 tests — projection correctness for the four most
//! common authoring patterns.
//!
//! Each test builds a tiny `Cx` arena, runs the projection, and asserts
//! against an inline snapshot. The snapshot format is the indented-text
//! formatter from [`a11y::snapshot`].

use a11y::{snapshot, tree};
use accesskit::Role;
use kernel::{Cx, container, fragment};

// hal is needed by sigil-a11y's public bridge trait but not
// named directly in this test file.
use hal as _;
// sigil-reactive (Wave 5.2 reactive_flow.rs dev-dep) + sigil
// (Phase 1.3.C live_regions.rs dev-dep) + crossbeam-channel (W5-A11Y
// AccessKitBridge's action-request channel, unused by this file's
// bounds-free `tree(&cx, None, &|_| None)` calls) — every integration
// test pulls all dev-deps, so we alias to silence
// `unused_crate_dependencies`. `layout` (RC-focus finding 1's
// `bounds_projection.rs` dev-dep) is unused here too.
use crossbeam_channel as _;
use layout as _;
use reactive as _;
use sigil as _;

#[test]
fn button_role_and_name() {
    let mut cx = Cx::new_root();
    let _ = container().role(Role::Button).label("Save").add(&mut cx);

    let update = tree(&cx, None, &|_| None);
    let text = snapshot(&update);

    assert_eq!(text, "[Role=Button, Name=\"Save\", Focused=true]\n");
}

#[test]
fn checkbox_role_surfaces() {
    let mut cx = Cx::new_root();
    let _ = container()
        .role(Role::CheckBox)
        .label("Accept terms")
        .add(&mut cx);

    let update = tree(&cx, None, &|_| None);
    let text = snapshot(&update);

    assert_eq!(
        text,
        "[Role=CheckBox, Name=\"Accept terms\", Focused=true]\n"
    );
}

#[test]
fn generic_container_default_role() {
    let mut cx = Cx::new_root();
    let _ = container().add(&mut cx);

    let update = tree(&cx, None, &|_| None);
    let text = snapshot(&update);

    assert_eq!(text, "[Role=GenericContainer, Focused=true]\n");
}

#[test]
fn fragment_reparents_children_in_projection() {
    // Fragment wraps a button; the projection should elide the fragment
    // and surface the button as the root.
    let mut cx = Cx::new_root();
    let _ = fragment()
        .child(container().role(Role::Button).label("Inner"))
        .add(&mut cx);

    let update = tree(&cx, None, &|_| None);
    let text = snapshot(&update);

    assert_eq!(text, "[Role=Button, Name=\"Inner\", Focused=true]\n");
}
