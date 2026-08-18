//! Phase 1.3 acceptance gate — live-region announcement reaches the
//! AccessKit projection.
//!
//! Predecessors:
//!   - Wave 1.3.A added [`sigil_kernel::Cx::announce`] +
//!     [`sigil_kernel::queue_announcement`] and lazily spawns a
//!     hidden live-region node on the first call.
//!   - Wave 1.3.B added the modal's rising-edge effect that pushes
//!     the dialog title through `queue_announcement` on every
//!     `false → true` transition of the `show` signal.
//!
//! This integration test is the gate that proves the full pipeline:
//! signal flip → effect → queue → drain → kernel live-region node →
//! [`sigil_a11y::tree`] projection → an [`accesskit::TreeUpdate`] node
//! whose `live()` is set and whose `label()` carries the announcement
//! text. This is what real screen readers consume; nothing about the
//! upstream kernel-side asserts proves the AccessKit side wires up.

#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

// Workspace-shape integration-test dev-deps that aren't named here.
// `sigil-reactive` is exercised through the modal's effect machinery
// and through `Runtime::scope`, but we don't name it directly.
// `crossbeam-channel` backs W5-A11Y's `AccessKitBridge`, unused by this
// file's bounds-free `tree(&cx, None, &|_| None)` calls.
use crossbeam_channel as _;
use sigil_hal as _;
use sigil_layout as _;
use sigil_reactive as _;

use accesskit::{Live, Node as AkNode, NodeId as AkNodeId, TreeUpdate};
use sigil_a11y::tree;
use sigil_kernel::{Cx, container};

use sigil::prelude::*;
use sigil::reactive::Runtime;

/// Walk every node in the [`TreeUpdate`] (regardless of position in the
/// tree) looking for the single live-region node.
///
/// The kernel adds the live region as a sibling root of any author tree
/// — so a plain depth-first walk from `tree.root` would miss it. We scan
/// the flat node list because the gate's question is "did the projection
/// surface a `Live::*` node at all?", not "is it positioned at a
/// particular depth?".
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

        // Rising edge: false → true. The modal's effect queues a polite
        // announcement carrying the dialog title; the host drain copies
        // the text + politeness onto the kernel live-region node.
        visible.set(true);
        cx.flush_announcements();

        // Now snapshot the AccessKit projection — the full pipeline
        // including the mapping-layer `set_live` + `set_label`.
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
    // Two consecutive direct `cx.announce` calls. The contract on the
    // kernel side is "overwrite, not append" — the AccessKit projection
    // must reflect that: exactly one live-region node, label = "bar".
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
    // Lazy-creation contract: an app that never announces pays zero
    // a11y-surface cost. The AccessKit tree must not contain any node
    // with `live()` set, and the projection must still include the
    // author's own widget so we know the snapshot is non-empty.
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
    // Bonus case: the assertive politeness path. `announce_assertive`
    // is the rare path (errors, security prompts), and the AccessKit
    // projection must distinguish it from polite so screen readers
    // interrupt the user appropriately.
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
