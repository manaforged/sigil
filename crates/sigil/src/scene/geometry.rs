use crate::geometry::{Point, Rect};
use crate::scene::command::Transform;

pub(crate) const IDENTITY: Transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

pub(crate) fn compose(parent: Transform, child: Transform) -> Transform {
    [
        [
            parent[0][0].mul_add(child[0][0], parent[0][1] * child[1][0]),
            parent[0][0].mul_add(child[0][1], parent[0][1] * child[1][1]),
            parent[0][0].mul_add(child[0][2], parent[0][1].mul_add(child[1][2], parent[0][2])),
        ],
        [
            parent[1][0].mul_add(child[0][0], parent[1][1] * child[1][0]),
            parent[1][0].mul_add(child[0][1], parent[1][1] * child[1][1]),
            parent[1][0].mul_add(child[0][2], parent[1][1].mul_add(child[1][2], parent[1][2])),
        ],
    ]
}

pub(crate) fn intersect_rect(left: Rect, right: Rect) -> Option<Rect> {
    let x0 = left.origin.x.max(right.origin.x);
    let y0 = left.origin.y.max(right.origin.y);
    let x1 = (left.origin.x + left.width).min(right.origin.x + right.width);
    let y1 = (left.origin.y + left.height).min(right.origin.y + right.height);
    (x1 > x0 && y1 > y0).then(|| Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0))
}
