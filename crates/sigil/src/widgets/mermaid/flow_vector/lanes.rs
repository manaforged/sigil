use super::{Area, LANE, OVERLAP, Point};

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
