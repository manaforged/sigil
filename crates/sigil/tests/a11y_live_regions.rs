#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, clippy::unwrap_used)]

use accesskit::{Live, Node as AkNode, NodeId as AkNodeId, TreeUpdate};
use sigil::a11y::tree;
use sigil::kernel::{Cx, container};

use sigil::prelude::*;
use sigil::reactive::Runtime;

fn find_live_node(update: &TreeUpdate) -> Option<(AkNodeId, &AkNode)> {
    update
        .nodes
        .iter()
        .find(|(_, node)| node.live().is_some())
        .map(|(id, node)| (*id, node))
}

fn count_live_nodes(update: &TreeUpdate) -> usize {
    update
        .nodes
        .iter()
        .filter(|(_, node)| node.live().is_some())
        .count()
}

#[test]
fn modal_open_announces_polite_in_accesskit_tree() {
    Runtime::scope("phase-1-3c::modal-polite-accesskit", || {
        let mut cx = Cx::new_root();
        let visible = signal(false);
        let _ = cx.add(modal(visible).title("File saved"));

        visible.set(true);
        cx.flush_announcements();

        let update = tree(&cx, None, &|_| None);
        let (_, live_node) = find_live_node(&update)
            .expect("modal rising edge must surface a live-region node in the AccessKit tree");

        assert_eq!(
            live_node.live(),
            Some(Live::Polite),
            "modal's default politeness is polite, per ARIA convention"
        );
        assert_eq!(
            live_node.label(),
            Some("File saved"),
            "AccessKit label must carry the dialog title verbatim"
        );
    });
}

#[test]
fn multiple_announces_overwrite_label_polite() {
    let mut cx = Cx::new_root();
    cx.announce("foo");
    cx.announce("bar");

    let update = tree(&cx, None, &|_| None);
    assert_eq!(
        count_live_nodes(&update),
        1,
        "consecutive announces share one node; we never accumulate one per call"
    );
    let (_, live_node) =
        find_live_node(&update).expect("after announce the projection has a live node");
    assert_eq!(
        live_node.label(),
        Some("bar"),
        "second announce wins — overwrite semantics, not append"
    );
    assert_eq!(
        live_node.live(),
        Some(Live::Polite),
        "polite announce keeps polite politeness"
    );
}

#[test]
fn no_announce_no_live_region_node() {
    let mut cx = Cx::new_root();
    let _ = cx.add(container().label("just a button"));

    let update = tree(&cx, None, &|_| None);
    assert!(
        !update.nodes.is_empty(),
        "sanity: author widget must still appear in the projection"
    );
    assert_eq!(
        count_live_nodes(&update),
        0,
        "no announce → no live-region node anywhere in the AccessKit tree"
    );
}

#[test]
fn announce_assertive_surfaces_assertive_in_accesskit_tree() {
    let mut cx = Cx::new_root();
    cx.announce_assertive("Connection lost");

    let update = tree(&cx, None, &|_| None);
    let (_, live_node) = find_live_node(&update)
        .expect("announce_assertive must create a live-region node visible in projection");
    assert_eq!(
        live_node.live(),
        Some(Live::Assertive),
        "assertive announce projects as Live::Assertive, distinct from Polite"
    );
    assert_eq!(
        live_node.label(),
        Some("Connection lost"),
        "label carries the urgent text verbatim"
    );
}

#[test]
fn announcement_tree_is_accepted_by_platform_consumer() {
    let mut cx = Cx::new_root();
    let _ = cx.add(button("Save"));
    cx.announce("saved");
    cx.flush_announcements();
    let consumed = accesskit_consumer::Tree::new(tree(&cx, None, &|_| None), false);
    assert!(consumed.state().root().children().count() >= 2);
}

#[test]
fn open_modal_tree_is_accepted_by_platform_consumer() {
    Runtime::scope("a11y-modal-consumer", || {
        let mut cx = Cx::new_root();
        let _ = cx.add(button("Save"));
        let visible = signal(false);
        let _ = cx.add(modal(visible).title("File saved"));
        visible.set(true);
        cx.flush_announcements();
        let consumed = accesskit_consumer::Tree::new(tree(&cx, None, &|_| None), false);
        assert!(consumed.state().root().children().count() >= 2);
    });
}
