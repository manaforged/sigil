#![allow(clippy::panic, clippy::unwrap_used)]

use crate::kernel::Live;

use super::*;

#[test]
fn announce_marks_live_region_polite_and_sets_label() {
    let mut cx = Cx::new_root();
    announce_then_flush(
        &mut cx,
        |text| super::announce(text, Live::Polite),
        "File saved",
    );

    let id = cx.live_region_id().expect("announce should create node");
    let node = cx.get(id).expect("live region node exists");

    assert_eq!(node.a11y.live, Some(Live::Polite));
    assert_label(node, "File saved");
    assert!(matches!(node.kind, NodeType::Container));
}

#[test]
fn announce_reuses_the_same_node_across_calls() {
    let mut cx = Cx::new_root();
    announce_then_flush(&mut cx, |text| super::announce(text, Live::Polite), "first");
    let first = cx.live_region_id().expect("created");
    announce_then_flush(
        &mut cx,
        |text| super::announce(text, Live::Polite),
        "second",
    );
    let second = cx.live_region_id().expect("still present");

    assert_eq!(first, second);
    assert_label(cx.get(second).expect("node"), "second");
}

#[test]
fn announce_assertive_sets_assertive_politeness() {
    let mut cx = Cx::new_root();
    announce_then_flush(
        &mut cx,
        |text| super::announce(text, Live::Assertive),
        "Connection lost",
    );

    let id = cx.live_region_id().expect("node created");
    let node = cx.get(id).expect("node exists");
    assert_eq!(node.a11y.live, Some(Live::Assertive));
}

#[test]
fn cx_without_announce_has_no_live_region() {
    let cx = Cx::new_root();
    assert!(cx.live_region_id().is_none());
}

fn assert_label(node: &Node, want: &str) {
    match &node.a11y.name {
        AccessibleName::Static(text) => assert_eq!(text, want),
        other => panic!("expected static label, got {other:?}"),
    }
}

fn announce_then_flush(cx: &mut Cx, post: fn(String), text: &str) {
    post(text.to_owned());
    cx.flush_announcements();
}
