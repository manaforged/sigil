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

pub fn queue_announcement(text: impl Into<String>, live: Live) {
    PENDING.with(|cell| cell.borrow_mut().push_back((text.into(), live)));
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

    pub fn announce(&mut self, text: impl Into<String>) {
        self.post_announcement(text.into(), Live::Polite);
    }

    pub fn announce_assertive(&mut self, text: impl Into<String>) {
        self.post_announcement(text.into(), Live::Assertive);
    }

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
        owner_id: crate::reactive::OwnerId,
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
#[path = "announce_tests.rs"]
mod tests;
