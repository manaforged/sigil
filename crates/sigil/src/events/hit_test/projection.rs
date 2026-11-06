use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, Node, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::Transform3D;
use crate::style::atom::SpaceStyle;
use crate::style::projection::projection;
use crate::style::{ClipShape, StyleAtom, StyleBinding, Transform2D};

fn node_projection(
    cx: &Cx,
    id: NodeId,
    binding: &StyleBinding,
    rect: Rect,
) -> Option<(Transform3D, bool)> {
    let flat = Transform2D::for_binding(binding);
    let space = SpaceStyle::for_binding(binding);
    projection(cx, id, (flat, &space), rect)
        .or_else(|| (!flat.is_identity()).then(|| (Transform3D::from(flat.matrix(rect)), false)))
}

pub(super) fn node_transform(
    cx: &Cx,
    id: NodeId,
    binding: &StyleBinding,
    rect: Rect,
) -> Option<Transform3D> {
    node_projection(cx, id, binding, rect).map(|(transform, _)| transform)
}

pub(super) fn to_local(
    cx: &Cx,
    id: NodeId,
    binding: &StyleBinding,
    rect: Rect,
    point: Point,
) -> Option<Point> {
    match node_projection(cx, id, binding, rect) {
        None => Some(point),
        Some((transform, hidden)) => transform.visible(hidden)?.unproject(point),
    }
}

fn clip_shape(binding: &StyleBinding) -> Option<ClipShape> {
    binding
        .cascade()
        .into_iter()
        .fold(None, |shape, atom| match atom {
            StyleAtom::ClipPath(value) => Some(value),
            _ => shape,
        })
}

pub(super) fn inside_clip_path(cx: &Cx, id: NodeId, node: &Node, position: Point) -> bool {
    let Some(shape) = node.get_extension::<StyleBinding>().and_then(clip_shape) else {
        return true;
    };
    let Some(rect) = node.get_extension::<LayoutResult>().map(LayoutResult::rect) else {
        return true;
    };
    super::local_position(cx, id, position).is_some_and(|point| shape.contains(rect, point))
}
