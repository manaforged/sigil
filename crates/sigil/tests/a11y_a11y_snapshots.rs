#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use accesskit::Role;
use sigil::a11y::{snapshot, tree};
use sigil::kernel::{Cx, container, fragment};

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
    let mut cx = Cx::new_root();
    let _ = fragment()
        .child(container().role(Role::Button).label("Inner"))
        .add(&mut cx);

    let update = tree(&cx, None, &|_| None);
    let text = snapshot(&update);

    assert_eq!(text, "[Role=Button, Name=\"Inner\", Focused=true]\n");
}

#[test]
fn removed_focus_target_falls_back_to_live_root() {
    let mut cx = Cx::new_root();
    let root = container().child(container()).add(&mut cx);
    let stale_focus = cx.nth_child(root, 0).unwrap_or(root);
    assert!(cx.remove(stale_focus));

    let update = tree(&cx, Some(stale_focus), &|_| None);

    assert_eq!(update.focus, root.to_accesskit());
    assert!(update.nodes.iter().any(|(id, _)| *id == update.focus));
}
