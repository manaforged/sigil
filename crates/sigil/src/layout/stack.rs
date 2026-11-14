use smallvec::SmallVec;

use crate::kernel::{Cx, Node, NodeId};
use crate::layout::{LayoutStyle, Position};
use crate::style::StyleBinding;

pub(crate) type Stack = SmallVec<[NodeId; 8]>;

type Level = (i16, bool);

const FLOW: Level = (0, false);

fn level(cx: &Cx, id: NodeId) -> Level {
    let Some(node) = cx.get(id) else {
        return FLOW;
    };
    let z = node
        .extension::<StyleBinding>()
        .map_or(0, StyleBinding::z_index);
    let sticky = node
        .extension::<LayoutStyle>()
        .is_some_and(|style| style.data().position == Position::Sticky);
    (z, sticky)
}

pub(crate) fn stacked(cx: &Cx, node: &Node) -> Option<Stack> {
    let children = node.children();
    let mut previous = (i16::MIN, false);
    let sorted = children.iter().all(|child| {
        let current = level(cx, *child);
        let ordered = current >= previous;
        previous = current;
        ordered
    });
    if sorted {
        return None;
    }
    let mut levels: SmallVec<[(Level, NodeId); 8]> = children
        .iter()
        .map(|child| (level(cx, *child), *child))
        .collect();
    levels.sort_by_key(|(key, _)| *key);
    Some(levels.into_iter().map(|(_, child)| child).collect())
}
