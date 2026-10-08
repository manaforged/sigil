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
    let mut found = Vec::new();
    for id in cx.top_layers().iter().copied() {
        if let Some(path) = path_under(cx, node, id) {
            found.push((id, path));
        }
    }
    found
}

fn path_under<'t>(cx: &'t Cx, node: &Node, id: NodeId) -> Option<Path<'t>> {
    let mut path = Path::new();
    let mut current = cx.parent(id)?;
    loop {
        let step = cx.get(current)?;
        if core::ptr::eq(step, node) {
            path.reverse();
            return Some(path);
        }
        if is_top_layer(step) || skips_paint(step) {
            return None;
        }
        path.push((current, step));
        current = cx.parent(current)?;
    }
}
