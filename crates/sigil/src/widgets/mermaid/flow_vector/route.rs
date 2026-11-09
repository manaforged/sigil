use crate::widgets::vector::PathNode;

use super::{Area, CORNER, DASH, DASH_GAP, HEAD, LANE, OVERLAP, Point};

fn shared(a0: f32, a1: f32, b0: f32, b1: f32) -> Option<f32> {
    let (lo, hi) = (a0.max(b0), a1.min(b1));
    let need = if a1 - a0 < 1.0 || b1 - b0 < 1.0 {
        0.0
    } else {
        OVERLAP
    };
    (hi - lo >= need).then(|| f32::midpoint(lo, hi))
}

pub(super) fn route(vertical: bool, from: Area, to: Area, areas: &[Area]) -> Vec<Point> {
    if vertical {
        if to.cy() < from.cy() {
            return lane_right(from, to, areas);
        }
        let (start, end) = ((from.cx(), from.bottom()), (to.cx(), to.y));
        if let Some(x) = shared(from.x, from.right(), to.x, to.right()) {
            return vec![(x, start.1), (x, end.1)];
        }
        let mid = f32::midpoint(start.1, end.1);
        vec![start, (start.0, mid), (end.0, mid), end]
    } else {
        if to.cx() < from.cx() {
            return lane_below(from, to, areas);
        }
        let (start, end) = ((from.right(), from.cy()), (to.x, to.cy()));
        if let Some(y) = shared(from.y, from.bottom(), to.y, to.bottom()) {
            return vec![(start.0, y), (end.0, y)];
        }
        let mid = f32::midpoint(start.0, end.0);
        vec![start, (mid, start.1), (mid, end.1), end]
    }
}

fn lane_right(from: Area, to: Area, areas: &[Area]) -> Vec<Point> {
    let (top, bottom) = (to.y, from.bottom());
    let lane = areas
        .iter()
        .filter(|area| area.bottom() >= top && area.y <= bottom)
        .map(|area| area.right())
        .fold(from.right().max(to.right()), f32::max)
        + LANE;
    vec![
        (from.right(), from.cy()),
        (lane, from.cy()),
        (lane, to.cy()),
        (to.right(), to.cy()),
    ]
}

fn lane_below(from: Area, to: Area, areas: &[Area]) -> Vec<Point> {
    let (left, right) = (to.x, from.right());
    let lane = areas
        .iter()
        .filter(|area| area.right() >= left && area.x <= right)
        .map(|area| area.bottom())
        .fold(from.bottom().max(to.bottom()), f32::max)
        + LANE;
    vec![
        (from.cx(), from.bottom()),
        (from.cx(), lane),
        (to.cx(), lane),
        (to.cx(), to.bottom()),
    ]
}

fn distinct(points: &[Point]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        let same = out
            .last()
            .is_some_and(|last| (last.0 - point.0).abs() < 0.5 && (last.1 - point.1).abs() < 0.5);
        if !same {
            out.push(*point);
        }
    }
    out
}

fn unit(from: Point, to: Point) -> (Point, f32) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy);
    if length < f32::EPSILON {
        return ((0.0, 0.0), 0.0);
    }
    ((dx / length, dy / length), length)
}

pub(in crate::widgets::mermaid) fn polyline(mut node: PathNode, points: &[Point]) -> PathNode {
    let points = distinct(points);
    let Some(first) = points.first() else {
        return node;
    };
    node = node.move_to(first.0, first.1);
    for window in points.windows(3) {
        let &[before, corner, after] = window else {
            continue;
        };
        let ((ux, uy), into) = unit(before, corner);
        let ((vx, vy), out) = unit(corner, after);
        let r = CORNER.min(into / 2.0).min(out / 2.0);
        node = node
            .line_to((-ux).mul_add(r, corner.0), (-uy).mul_add(r, corner.1))
            .quad_to(
                corner.0,
                corner.1,
                vx.mul_add(r, corner.0),
                vy.mul_add(r, corner.1),
            );
    }
    match points.last() {
        Some(last) if points.len() > 1 => node.line_to(last.0, last.1),
        _ => node,
    }
}

pub(in crate::widgets::mermaid) fn dashed_line(mut node: PathNode, points: &[Point]) -> PathNode {
    for pair in distinct(points).windows(2) {
        let &[start, finish] = pair else {
            continue;
        };
        let ((ux, uy), length) = unit(start, finish);
        let steps = std::iter::successors(Some(0.0_f32), |at| Some(at + DASH + DASH_GAP));
        for at in steps.take_while(|at| *at < length) {
            let end = (at + DASH).min(length);
            node = node
                .move_to(ux.mul_add(at, start.0), uy.mul_add(at, start.1))
                .line_to(ux.mul_add(end, start.0), uy.mul_add(end, start.1));
        }
    }
    node
}

pub(super) fn arrow_head(node: PathNode, points: &mut [Point]) -> PathNode {
    let count = points.len();
    let (Some(tip), Some(before)) = (
        points.last().copied(),
        distinct(points).iter().rev().nth(1).copied(),
    ) else {
        return node;
    };
    let ((ux, uy), length) = unit(before, tip);
    let head = HEAD.min(length);
    let base = ((-ux).mul_add(head, tip.0), (-uy).mul_add(head, tip.1));
    if let Some(last) = points.get_mut(count.saturating_sub(1)) {
        *last = base;
    }
    barb(node, tip, base, (ux, uy), head * 0.55)
}

pub(in crate::widgets::mermaid) fn barb(
    node: PathNode,
    tip: Point,
    base: Point,
    dir: Point,
    half: f32,
) -> PathNode {
    let (ux, uy) = dir;
    node.move_to(tip.0, tip.1)
        .line_to((-uy).mul_add(half, base.0), ux.mul_add(half, base.1))
        .line_to(uy.mul_add(half, base.0), (-ux).mul_add(half, base.1))
        .close()
}

pub(super) fn middle(points: &[Point]) -> Point {
    let points = distinct(points);
    let count = points.len();
    let (Some(a), Some(b)) = (
        points.get(count.saturating_sub(1) / 2),
        points.get(count / 2),
    ) else {
        return (0.0, 0.0);
    };
    if count.is_multiple_of(2) {
        (f32::midpoint(a.0, b.0), f32::midpoint(a.1, b.1))
    } else {
        *a
    }
}
