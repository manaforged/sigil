use smallvec::SmallVec;

use crate::kernel::{Cx, Node, NodeId};
use crate::scene::style_adapter::render_style_for;

use super::{EmitWalk, Inherited, child_inherited, is_top_layer, skips_paint};

type Path<'t> = SmallVec<[(NodeId, &'t Node); 16]>;

pub(super) fn defer_top_layer<'t>(
    walk: &mut EmitWalk<'_, 't>,
    node: &Node,
    context: impl FnOnce() -> Inherited<'t>,
) {
    let found = find_top_layer(walk.cx, node);
    if found.is_empty() {
        return;
    }
    let base = context();
    for (id, path) in found {
        let inherited = path.iter().fold(base.clone(), |outer, &(step, step_node)| {
            let style = render_style_for(step, step_node, outer.theme, outer.text_color);
            child_inherited(step_node, &outer, &style)
        });
        walk.deferred.push((id, inherited, None));
    }
}

fn find_top_layer<'t>(cx: &'t Cx, node: &Node) -> Vec<(NodeId, Path<'t>)> {
    let mut found = Vec::new();
    let mut stack: SmallVec<[(NodeId, usize); 32]> =
        node.children().iter().rev().map(|id| (*id, 0)).collect();
    let mut path = Path::new();
    while let Some((id, depth)) = stack.pop() {
        path.truncate(depth);
        let Some(child) = cx.get(id) else {
            continue;
        };
        if is_top_layer(child) {
            found.push((id, path.clone()));
        } else if !child.children().is_empty() && !skips_paint(child) {
            path.push((id, child));
            stack.extend(
                child
                    .children()
                    .iter()
                    .rev()
                    .map(|grandchild| (*grandchild, depth + 1)),
            );
        }
    }
    found
}
