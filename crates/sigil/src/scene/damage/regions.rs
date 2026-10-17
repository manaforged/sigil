use super::Damage;
use crate::geometry::{Point, Rect};
use crate::scene::geometry::intersect_rect;
use crate::scene::state::snap_out;

pub(super) fn cells(
    inks: &[Rect],
    area: Rect,
    (width, height): (f32, f32),
    antialias: f32,
) -> Damage<Vec<Rect>> {
    let mut merged: Vec<Rect> = Vec::new();
    for rect in inks
        .iter()
        .map(|ink| unpadded(*ink, antialias))
        .filter_map(|ink| intersect_rect(ink, area))
        .map(|rect| snap_cells(rect, area.origin, (width, height)))
    {
        absorb(&mut merged, rect);
    }
    if merged.is_empty() {
        return Damage::Unchanged;
    }
    Damage::Regions(merged)
}

fn unpadded(ink: Rect, antialias: f32) -> Rect {
    let x = if ink.width > 2.0 * antialias {
        antialias
    } else {
        0.0
    };
    let y = if ink.height > 2.0 * antialias {
        antialias
    } else {
        0.0
    };
    Rect::new(
        Point::new(ink.origin.x + x, ink.origin.y + y),
        (-2.0_f32).mul_add(x, ink.width),
        (-2.0_f32).mul_add(y, ink.height),
    )
}

fn snap_cells(rect: Rect, origin: Point, (width, height): (f32, f32)) -> Rect {
    let across = snap_out(rect, origin, width.recip());
    let down = snap_out(rect, origin, height.recip());
    Rect::new(
        Point::new(across.origin.x, down.origin.y),
        across.width,
        down.height,
    )
}

fn absorb(merged: &mut Vec<Rect>, rect: Rect) {
    let mut rect = rect;
    while let Some(index) = merged.iter().position(|region| touches(*region, rect)) {
        rect = rect.union(merged.swap_remove(index));
    }
    merged.push(rect);
}

fn touches(a: Rect, b: Rect) -> bool {
    a.origin.x <= b.origin.x + b.width
        && b.origin.x <= a.origin.x + a.width
        && a.origin.y <= b.origin.y + b.height
        && b.origin.y <= a.origin.y + a.height
}
