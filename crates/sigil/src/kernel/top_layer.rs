use std::rc::Rc;

use crate::kernel::{Cx, NodeId, TopLayer};

pub(crate) type TopLayerMemo = std::cell::RefCell<Option<(u64, Rc<[NodeId]>)>>;

impl Cx {
    /// Returns the top-layer nodes and all their ancestors, sorted by id.
    #[must_use]
    pub(crate) fn top_layers(&self) -> Rc<[NodeId]> {
        let generation = self.mutation_generation;
        if let Some((seen, ids)) = self.top_layer_memo.borrow().as_ref()
            && *seen == generation
        {
            return Rc::clone(ids);
        }
        let mut reach = Vec::new();
        for (key, node) in &self.nodes {
            if node.extension::<TopLayer>().is_some() {
                reach.extend(std::iter::successors(Some(NodeId::from_key(key)), |id| {
                    self.parent(*id)
                }));
            }
        }
        reach.sort_unstable();
        reach.dedup();
        let ids: Rc<[NodeId]> = reach.into();
        *self.top_layer_memo.borrow_mut() = Some((generation, Rc::clone(&ids)));
        ids
    }
}
