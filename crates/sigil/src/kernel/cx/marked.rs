use ahash::AHashSet;
use smallvec::SmallVec;

use crate::kernel::{Cx, Node, NodeId};
use crate::layout::{Popup, ViewportOverlay};
use crate::style::StyleBinding;

#[derive(Default)]
pub(crate) struct Marked {
    ids: AHashSet<NodeId>,
}

impl Marked {
    pub(super) fn touch(&mut self, id: NodeId) {
        self.ids.insert(id);
    }
}

fn is_marked(node: &Node) -> bool {
    node.a11y.focus_scope.is_some()
        || node.get_extension::<ViewportOverlay>().is_some()
        || node.get_extension::<Popup>().is_some()
}

fn hides(node: &Node) -> bool {
    node.get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
}

impl Cx {
    pub(crate) fn marked_preorder(&self, roots: &[NodeId]) -> Vec<NodeId> {
        let mut marked = self.marked.borrow_mut();
        marked.ids.retain(|id| self.get(*id).is_some_and(is_marked));
        let mut hits: Vec<(SmallVec<[usize; 16]>, NodeId)> = marked
            .ids
            .iter()
            .filter_map(|id| self.tree_path(*id, roots).map(|path| (path, *id)))
            .collect();
        hits.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        hits.into_iter().map(|(_, id)| id).collect()
    }

    fn tree_path(&self, id: NodeId, roots: &[NodeId]) -> Option<SmallVec<[usize; 16]>> {
        let mut path = SmallVec::new();
        let mut current = id;
        loop {
            let node = self.get(current)?;
            if hides(node) {
                return None;
            }
            // complexity: bounded by the handful of window roots
            if let Some(index) = roots.iter().position(|root| *root == current) {
                path.push(index);
                path.reverse();
                return Some(path);
            }
            let parent = node.parent?;
            let index = self
                .get(parent)?
                .children
                .iter()
                .position(|child| *child == current)?;
            path.push(index);
            current = parent;
        }
    }
}
