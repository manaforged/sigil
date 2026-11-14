use smallvec::SmallVec;

use crate::kernel::{Cx, Node, NodeId};
use crate::layout::{LayoutStyle, Position};

pub(crate) type Stack = SmallVec<[NodeId; 8]>;

fn sticky(cx: &Cx, id: NodeId) -> bool {
    cx.get(id)
        .and_then(|node| node.extension::<LayoutStyle>())
        .is_some_and(|style| style.data().position == Position::Sticky)
}

pub(crate) fn stacked(cx: &Cx, node: &Node) -> Option<Stack> {
    let children = node.children();
    if !children.iter().any(|child| sticky(cx, *child)) {
        return None;
    }
    let flow = children.iter().copied().filter(|child| !sticky(cx, *child));
    let held = children.iter().copied().filter(|child| sticky(cx, *child));
    Some(flow.chain(held).collect())
}
