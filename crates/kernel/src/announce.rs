//! Live-region announcement API.
//!
//! Application code calls [`Cx::announce`] to post a message that
//! assistive technology should read out. The kernel maintains a single
//! lazily-created live-region node per [`Cx`]: a [`NodeType::Container`]
//! marked `live = Some(accesskit::Live::Polite)` (or `Assertive` for
//! the urgent variant). The node's [`AccessibleName::Static`] label
//! carries the announcement text, and the a11y mapping layer picks it
//! up on the next [`TreeUpdate`].
//!
//! The live-region node is created on the first call. Apps that never
//! announce pay no cost.
//!
//! # Deferred queue
//!
//! Reactive effects cannot hold a `&mut Cx` borrow, so widgets that
//! need to announce in response to a signal change push through
//! [`queue_announcement`] — a thread-local fifo drained by
//! [`Cx::flush_announcements`]. Tests and frame drivers call the drain
//! on the same `Cx` that owns the live region.

use std::cell::RefCell;
use std::collections::VecDeque;

use accesskit::Live;
use sigil_reactive::Owner;

use crate::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType};

thread_local! {
    static PENDING: RefCell<VecDeque<(String, Live)>> = const {
        RefCell::new(VecDeque::new())
    };
}

/// Queue an announcement for the next [`Cx::flush_announcements`] drain.
///
/// Reactive effects use this seam: they observe a signal, decide a
/// rising-edge condition has fired, and push the text + politeness here
/// without needing `&mut Cx`. The drain runs against the `Cx` that owns
/// the live region, which calls [`Cx::announce`] / [`Cx::announce_assertive`]
/// per queued entry in FIFO order.
pub fn queue_announcement(text: impl Into<String>, live: Live) {
    PENDING.with(|cell| cell.borrow_mut().push_back((text.into(), live)));
}

impl Cx {
    /// Drain queued announcements from [`queue_announcement`] and post
    /// each one. FIFO order; politeness is honored per entry.
    ///
    /// Frame drivers and tests call this after running effects that may
    /// have queued announcements. No-op when the queue is empty.
    pub fn flush_announcements(&mut self) {
        loop {
            let next = PENDING.with(|cell| cell.borrow_mut().pop_front());
            let Some((text, live)) = next else {
                return;
            };
            self.post_announcement(text, live);
        }
    }

    /// Post a polite announcement to assistive technology.
    ///
    /// The text replaces any prior pending announcement. Politeness
    /// follows the ARIA-style convention: assistive tech waits for an
    /// idle moment before speaking, and does not interrupt the user.
    ///
    /// On the first call, this lazily creates a hidden live-region node
    /// at the root of the tree. Subsequent calls update that same node.
    pub fn announce(&mut self, text: impl Into<String>) {
        self.post_announcement(text.into(), Live::Polite);
    }

    /// Post an assertive announcement to assistive technology.
    ///
    /// Use sparingly: assertive announcements interrupt the user's
    /// current task. Reserve for errors, security prompts, and other
    /// signals the user must acknowledge before continuing.
    pub fn announce_assertive(&mut self, text: impl Into<String>) {
        self.post_announcement(text.into(), Live::Assertive);
    }

    /// Node id of the live region, if a prior `announce` call created one.
    #[must_use]
    pub const fn live_region_id(&self) -> Option<NodeId> {
        self.live_region
    }

    fn post_announcement(&mut self, text: String, politeness: Live) {
        let id = self.ensure_live_region(politeness);
        if let Some(node) = self.get_mut(id) {
            node.a11y.name = AccessibleName::Static(text);
            node.a11y.live = Some(politeness);
        }
    }

    fn ensure_live_region(&mut self, politeness: Live) -> NodeId {
        if let Some(id) = self.live_region {
            return id;
        }
        let a11y = A11yProps {
            live: Some(politeness),
            hittable: false,
            ..A11yProps::default()
        };
        let id = self.spawn_live_region(a11y);
        self.live_region = Some(id);
        id
    }

    fn spawn_live_region(&mut self, a11y: A11yProps) -> NodeId {
        let parent_owner = self.root_owner();
        let owner = Owner::new("sigil-kernel::live-region", Some(parent_owner));
        let id = owner.with(|| {
            let owner_id = Owner::current().expect("live-region owner is current inside `with`");
            self.insert_live_region_node(a11y, owner_id)
        });
        self.register_live_region_owner(id, owner);
        id
    }

    fn insert_live_region_node(
        &mut self,
        a11y: A11yProps,
        owner_id: sigil_reactive::OwnerId,
    ) -> NodeId {
        NodeId(
            self.nodes
                .insert(Node::new(NodeType::Container, a11y, owner_id)),
        )
    }

    fn register_live_region_owner(&mut self, id: NodeId, owner: Owner) {
        self.owner_for_node.insert(id, owner);
        self.roots.push(id);
        self.dirty.insert(id);
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::unwrap_used)]
mod tests {
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
}
