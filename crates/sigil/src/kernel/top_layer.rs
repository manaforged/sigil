use std::rc::Rc;

use crate::kernel::{Cx, NodeId, TopLayer};

pub(crate) type TopLayerMemo = std::cell::RefCell<Option<(u64, Rc<[NodeId]>)>>;

impl Cx {
    #[must_use]
    pub(crate) fn top_layers(&self) -> Rc<[NodeId]> {
        let generation = self.mutation_generation;
        if let Some((seen, ids)) = self.top_layer_memo.borrow().as_ref()
            && *seen == generation
        {
            return Rc::clone(ids);
        }
        let ids: Rc<[NodeId]> = self
            .nodes
            .iter()
            .filter(|(_, node)| node.get_extension::<TopLayer>().is_some())
            .map(|(key, _)| NodeId::from_key(key))
            .collect();
        *self.top_layer_memo.borrow_mut() = Some((generation, Rc::clone(&ids)));
        ids
    }
}
