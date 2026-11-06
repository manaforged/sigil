use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::Transform3D;

use super::atom::{Backface, SpaceStyle};
use super::{StyleAtom, StyleBinding, Transform2D};

impl SpaceStyle {
    pub(crate) fn for_binding(binding: &StyleBinding) -> Self {
        let mut style = Self::FLAT;
        for atom in binding.cascade() {
            if let StyleAtom::Space(atom) = atom {
                style.apply(atom);
            }
        }
        if let Some(overlay) = binding.animator_overlay() {
            style.overlay(overlay);
        }
        style
    }

    fn perspective_at(&self, rect: Rect) -> Option<Transform3D> {
        let distance = self.perspective?;
        let [ox, oy] = self.perspective_origin.map(|value| f32::from(value) / 10_000.0);
        let origin = Point::new(
            ox.mul_add(rect.width, rect.origin.x),
            oy.mul_add(rect.height, rect.origin.y),
        );
        let flat = Transform3D::IDENTITY;
        Some(flat.around(origin, flat.perspective(distance)))
    }
}

impl Transform2D {
    pub(crate) fn matrix3d(self, rect: Rect, space: &SpaceStyle) -> Transform3D {
        let flat = Transform3D::IDENTITY;
        let [x, y, z] = space.turn;
        let turn = flat.translate(0.0, 0.0, space.depth).rotate_x(x).rotate_y(y).rotate_z(z);
        Transform3D::from(self.matrix(rect)).then(flat.around(self.pivot(rect), turn))
    }
}

fn parent_perspective(cx: &Cx, id: NodeId) -> Option<Transform3D> {
    let parent = cx.get(cx.parent(id)?)?;
    let rect = parent.get_extension::<LayoutResult>()?.rect();
    SpaceStyle::for_binding(parent.get_extension::<StyleBinding>()?).perspective_at(rect)
}

pub(crate) fn projection(
    cx: &Cx,
    id: NodeId,
    (flat, space): (Transform2D, &SpaceStyle),
    rect: Rect,
) -> Option<(Transform3D, bool)> {
    if space.is_flat() {
        return None;
    }
    let own = flat.matrix3d(rect, space);
    let transform = parent_perspective(cx, id).map_or(own, |view| view.then(own));
    Some((transform, space.backface == Backface::Hidden))
}
