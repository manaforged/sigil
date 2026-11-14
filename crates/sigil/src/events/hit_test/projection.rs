use crate::geometry::Point;
use crate::kernel::{Cx, Node, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::Transform3D;
use crate::style::projection::{in_context, node_plane};
use crate::style::{ClipShape, StyleAtom, StyleBinding};

pub(super) fn node_transform(cx: &Cx, id: NodeId) -> Option<Transform3D> {
    node_plane(cx, id).map(|(transform, _)| transform)
}

fn to_local(cx: &Cx, id: NodeId, point: Point) -> Option<Point> {
    match node_plane(cx, id) {
        None => Some(point),
        Some((transform, hidden)) => transform.visible(hidden)?.unproject(point),
    }
}

pub(super) type Step = (Option<Point>, Option<Point>);

pub(super) fn descend(
    cx: &Cx,
    id: NodeId,
    (moved, linked): (bool, bool),
    (base, local): Step,
) -> Step {
    let context = linked && in_context(cx, id);
    let from = if context { base } else { local };
    let to = if moved || context {
        from.and_then(|point| to_local(cx, id, point))
    } else {
        from
    };
    (from, to)
}

fn clip_shape(binding: &StyleBinding) -> Option<ClipShape> {
    binding.cascade().fold(None, |shape, atom| match atom {
        StyleAtom::ClipPath(value) => Some(value),
        _ => shape,
    })
}

pub(super) fn inside_clip_path(cx: &Cx, id: NodeId, node: &Node, position: Point) -> bool {
    let Some(shape) = node.extension::<StyleBinding>().and_then(clip_shape) else {
        return true;
    };
    let Some(rect) = node.extension::<LayoutResult>().map(LayoutResult::rect) else {
        return true;
    };
    super::local_position(cx, id, position).is_some_and(|point| shape.contains(rect, point))
}
