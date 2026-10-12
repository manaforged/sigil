use std::rc::Rc;

use crate::reactive::effect;

use super::super::patterns::Disclosure;
use super::{NodesFn, TreeKey, TreeNode, TreeState};

#[derive(Clone)]
pub struct TreeRow<K: TreeKey> {
    pub id: K,
    pub depth: u16,
    pub disclosure: Disclosure,
    pub selected: bool,
    pub parent: Option<usize>,
    pub(super) node: Rc<TreeNode<K>>,
}

impl<K: TreeKey> TreeRow<K> {
    #[must_use]
    pub const fn has_children(&self) -> bool {
        matches!(
            self.disclosure,
            Disclosure::Collapsed | Disclosure::Expanded
        )
    }
}

impl<K: TreeKey> PartialEq for TreeRow<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.depth == other.depth
            && self.disclosure == other.disclosure
            && self.selected == other.selected
            && self.parent == other.parent
            && Rc::ptr_eq(&self.node, &other.node)
    }
}

pub(super) fn bind<K: TreeKey>(nodes: NodesFn<K>, state: &TreeState<K>) {
    let (rows, expanded, selected) = (state.rows, state.expanded, state.selected);
    effect(move || {
        let (roots, open, current) = (nodes(), expanded.get(), selected.get());
        let mut out = Vec::new();
        Walk {
            open: &open,
            selected: current.as_ref(),
        }
        .visit(&roots, 0, None, &mut out);
        rows.update(|value| *value = out);
    });
}

struct Walk<'a, K: TreeKey> {
    open: &'a [K],
    selected: Option<&'a K>,
}

impl<K: TreeKey> Walk<'_, K> {
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
