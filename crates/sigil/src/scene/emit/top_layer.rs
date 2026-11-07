use smallvec::SmallVec;

use crate::kernel::{Cx, Node, NodeId};
use crate::scene::style_adapter::render_style_for;

use super::{EmitWalk, Inherited, child_inherited, is_top_layer, skips_paint};

type Path<'t> = SmallVec<[(NodeId, &'t Node); 16]>;

impl<'t> EmitWalk<'_, 't> {
    pub(super) fn defer_top_layer(&mut self, node: &Node, context: impl FnOnce() -> Inherited<'t>) {
        let found = find_top_layer(self.cx, node);
        if found.is_empty() {
            return;
        }
        let base = context();
        for (id, path) in found {
            let inherited = path.iter().fold(base.clone(), |outer, &(step, step_node)| {
                let style = render_style_for(step, step_node, outer.theme, outer.text_color);
                child_inherited(step_node, &outer, &style)
            });
            self.deferred.push((id, inherited, None));
        }
    }
}

fn find_top_layer<'t>(cx: &'t Cx, node: &Node) -> Vec<(NodeId, Path<'t>)> {
    let reach = cx.top_layers();
    let mut found = Vec::new();
    collect(cx, &reach, node, &mut Path::new(), &mut found);
    found.sort_by_key(|(id, _)| *id);
    found
}

fn collect<'t>(
    cx: &'t Cx,
    reach: &[NodeId],
    node: &Node,
    path: &mut Path<'t>,
    found: &mut Vec<(NodeId, Path<'t>)>,
) {
    for &id in node.children() {
        let Some(child) = reach.binary_search(&id).is_ok().then(|| cx.get(id)).flatten() else {
            continue;
        };
        if is_top_layer(child) {
            found.push((id, path.clone()));
        } else if !skips_paint(child) {
            path.push((id, child));
            collect(cx, reach, child, path, found);
            path.pop();
        }
    }
}
