use std::cell::RefCell;
use std::collections::VecDeque;

use crate::kernel::Live;
use crate::reactive::Owner;

use crate::kernel::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType};

thread_local! {
    static PENDING: RefCell<VecDeque<(String, Live)>> = const {
        RefCell::new(VecDeque::new())
    };
}

/// Queues a screen-reader announcement with this politeness.
pub fn announce(text: impl Into<String>, politeness: Live) {
    PENDING.with(|cell| cell.borrow_mut().push_back((text.into(), politeness)));
}

impl Cx {
    /// Move the text that `announce` queued into the live region.
    pub fn flush_announcements(&mut self) {
        loop {
            let next = PENDING.with(|cell| cell.borrow_mut().pop_front());
            let Some((text, live)) = next else {
                return;
            };
            self.post_announcement(text, live);
        }
    }

    #[cfg(any(test, feature = "kittest"))]
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
        self.delta.record(id, crate::kernel::DeltaKind::Paint);
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
        let owner = Owner::new("sigil-kernel::live-region", Some(self.root_owner()));
        let id = owner.with(|| {
            NodeId(
                self.nodes
                    .insert(Node::new(NodeType::Container, a11y, owner)),
            )
        });
        self.owner_for_node.insert(id, owner);
        self.roots.push(id);
        self.dirty.insert(id);
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
        self.live_region = Some(id);
        id
    }
}

#[cfg(test)]
#[path = "announce_tests.rs"]
mod tests;
