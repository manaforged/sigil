use std::rc::Rc;

use crate::reactive::effect;

use super::super::patterns::Disclosure;
use core::hash::Hash;

use super::{NodesFn, TreeNode, TreeState};

/// One visible row of a flattened tree.
#[derive(Clone)]
pub(crate) struct TreeRow<K: Clone + Eq + Hash + 'static> {
    /// The id.
    pub id: K,
    /// The depth.
    pub depth: u16,
    /// The disclosure.
    pub disclosure: Disclosure,
    /// Whether selected.
    pub selected: bool,
    /// The parent, if any.
    pub parent: Option<usize>,
    pub(super) node: Rc<TreeNode<K>>,
}

impl<K: Clone + Eq + Hash + 'static> TreeRow<K> {
    /// Returns whether this tree row has children.
    #[must_use]
    pub(crate) const fn has_children(&self) -> bool {
        matches!(
            self.disclosure,
            Disclosure::Collapsed | Disclosure::Expanded
        )
    }
}

impl<K: Clone + Eq + Hash + 'static> PartialEq for TreeRow<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.depth == other.depth
            && self.disclosure == other.disclosure
            && self.selected == other.selected
            && self.parent == other.parent
            && Rc::ptr_eq(&self.node, &other.node)
    }
}

pub(super) fn bind<K: Clone + Eq + Hash + 'static>(nodes: NodesFn<K>, state: &TreeState<K>) {
    let (rows, expanded, selected) = (state.rows, state.expanded, state.selected);
    effect(move || {
        let (roots, open, current) = (nodes(), expanded.get(), selected.get());
        let mut out = Vec::new();
        TreeWalk {
            open: &open,
            selected: current.as_ref(),
        }
        .visit(&roots, 0, None, &mut out);
        rows.update(|value| *value = out);
    });
}

struct TreeWalk<'a, K: Clone + Eq + Hash + 'static> {
    open: &'a [K],
    selected: Option<&'a K>,
}

impl<K: Clone + Eq + Hash + 'static> TreeWalk<'_, K> {
    fn visit(
        &self,
        nodes: &[TreeNode<K>],
        depth: u16,
        parent: Option<usize>,
        out: &mut Vec<TreeRow<K>>,
    ) {
        for node in nodes {
            let index = out.len();
            let open = self.open.contains(&node.id);
            let disclosure = match (node.children.is_empty(), open) {
                (true, _) => Disclosure::Leaf,
                (false, true) => Disclosure::Expanded,
                (false, false) => Disclosure::Collapsed,
            };
            out.push(TreeRow {
                id: node.id.clone(),
                depth,
                disclosure,
                selected: self.selected == Some(&node.id),
                parent,
                node: Rc::new(TreeNode {
                    id: node.id.clone(),
                    label: node.label.clone(),
                    leading: node.leading.clone(),
                    status: node.status.clone(),
                    detail: node.detail.clone(),
                    tone: node.tone,
                    children: Vec::new(),
                }),
            });
            if disclosure == Disclosure::Expanded {
                self.visit(&node.children, depth.saturating_add(1), Some(index), out);
            }
        }
    }
}
