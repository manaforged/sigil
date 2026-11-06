use resvg::usvg::{Group, Node, Tree};

/// The most scene commands one picture lowers to before it is drawn as one resvg bitmap.
pub(super) const MAX_COMMANDS: usize = 65_536;
const MAX_NODES: usize = 16_384;

/// Whether the nodes of `tree`, with every clip, mask, pattern and text subtree counted at each use, fit the budget.
pub(super) fn fits(tree: &Tree) -> bool {
    let mut left = MAX_NODES;
    within(tree.root(), &mut left)
}

fn within(group: &Group, left: &mut usize) -> bool {
    for node in group.children() {
        let Some(rest) = left.checked_sub(1) else {
            return false;
        };
        *left = rest;
        let mut fits = true;
        node.subroots(|root| fits = fits && within(root, left));
        if !fits {
            return false;
        }
        if let Node::Group(child) = node
            && !within(child, left)
        {
            return false;
        }
    }
    true
}
