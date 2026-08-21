#![allow(clippy::panic, clippy::unwrap_used)]

use accesskit::Live;

use super::*;

#[test]
fn announce_marks_live_region_polite_and_sets_label() {
    let mut cx = Cx::new_root();
    cx.announce("File saved");

    let id = cx.live_region_id().expect("announce should create node");
    let node = cx.get(id).expect("live region node exists");

    assert_eq!(node.a11y.live, Some(Live::Polite));
    match &node.a11y.name {
        AccessibleName::Static(text) => assert_eq!(text, "File saved"),
        other => panic!("expected static label, got {other:?}"),
    }
    assert!(matches!(node.kind, NodeType::Container));
}

#[test]
fn announce_reuses_the_same_node_across_calls() {
    let mut cx = Cx::new_root();
    cx.announce("first");
    let first = cx.live_region_id().expect("created");
    cx.announce("second");
    let second = cx.live_region_id().expect("still present");

    assert_eq!(first, second);
    let node = cx.get(second).expect("node");
    match &node.a11y.name {
        AccessibleName::Static(text) => assert_eq!(text, "second"),
        other => panic!("expected static label, got {other:?}"),
    }
}

#[test]
fn announce_assertive_sets_assertive_politeness() {
    let mut cx = Cx::new_root();
    cx.announce_assertive("Connection lost");

    let id = cx.live_region_id().expect("node created");
    let node = cx.get(id).expect("node exists");
    assert_eq!(node.a11y.live, Some(Live::Assertive));
}

#[test]
fn cx_without_announce_has_no_live_region() {
    let cx = Cx::new_root();
    assert!(cx.live_region_id().is_none());
}
