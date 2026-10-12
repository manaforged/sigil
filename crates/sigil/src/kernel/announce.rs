use std::cell::RefCell;
use std::collections::VecDeque;

use crate::reactive::Owner;
use accesskit::Live;

use crate::kernel::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType};

thread_local! {
    static PENDING: RefCell<VecDeque<(String, Live)>> = const {
        RefCell::new(VecDeque::new())
    };
}

pub(crate) fn queue_announcement(text: impl Into<String>, live: Live) {
    PENDING.with(|cell| cell.borrow_mut().push_back((text.into(), live)));
}

pub fn announce(text: impl Into<String>) {
    queue_announcement(text, Live::Polite);
}

pub fn announce_assertive(text: impl Into<String>) {
    queue_announcement(text, Live::Assertive);
}

impl Cx {
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
                    .insert(Node::new(NodeType::Container, a11y, owner.id())),
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
