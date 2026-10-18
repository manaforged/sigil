use core::ops::Range;

use super::{Damage, FullReason};
use crate::geometry::{Point, Rect};
use crate::scene::geometry::intersect_rect;
use crate::scene::state::snap_out;

const MAX_CELL_WINDOWS: usize = 64;
const MAX_CELL_SHARE: f32 = 0.5;

pub(super) fn cells(
    inks: &[Rect],
    area: Rect,
    (width, height): (f32, f32),
    antialias: f32,
) -> Damage<Vec<Rect>> {
    let limit = area.width * area.height * MAX_CELL_SHARE;
    let mut rows: Vec<Vec<Range<usize>>> = Vec::new();
    let mut covered = 0_usize;
    for rect in inks
        .iter()
        .map(|ink| unpadded(*ink, antialias))
        .filter_map(|ink| intersect_rect(ink, area))
        .map(|rect| snap_cells(rect, area.origin, (width, height)))
    {
        let (xs, ys) = cell_range(rect, area.origin, (width, height));
        if xs.is_empty() || ys.is_empty() {
            continue;
        }
        if rows.len() < ys.end {
            rows.resize_with(ys.end, Vec::new);
        }
        for row in rows.get_mut(ys).into_iter().flatten() {
            covered += insert(row, xs.clone());
        }
        if to_f32(covered) * width * height > limit {
            return Damage::Full(FullReason::RegionShare);
        }
    }
    join(&rows, area.origin, (width, height))
}

fn insert(row: &mut Vec<Range<usize>>, span: Range<usize>) -> usize {
    let low = row.partition_point(|open| open.end < span.start);
    let high = row.partition_point(|open| open.start <= span.end).max(low);
    let mut merged = span;
    let mut removed = 0;
    for open in row.get(low..high).unwrap_or_default() {
        merged = merged.start.min(open.start)..merged.end.max(open.end);
        removed += open.len();
    }
    let added = merged.len().saturating_sub(removed);
    row.splice(low..high, core::iter::once(merged));
    added
}

fn join(
    rows: &[Vec<Range<usize>>],
    origin: Point,
    (width, height): (f32, f32),
) -> Damage<Vec<Rect>> {
    let mut windows: Vec<Rect> = Vec::new();
    let mut open: Vec<(Range<usize>, usize)> = Vec::new();
    for y in 0..=rows.len() {
        let line = rows.get(y).map_or(&[][..], Vec::as_slice);
        if open.iter().map(|(span, _)| span).eq(line.iter()) {
            continue;
        }
        for (span, top) in std::mem::take(&mut open) {
            windows.push(Rect::new(
                Point::new(
                    to_f32(span.start).mul_add(width, origin.x),
                    to_f32(top).mul_add(height, origin.y),
                ),
                to_f32(span.len()) * width,
                to_f32(y - top) * height,
            ));
            if windows.len() > MAX_CELL_WINDOWS {
                return Damage::Full(FullReason::RegionShare);
            }
        }
        open.extend(line.iter().map(|span| (span.clone(), y)));
    }
    if windows.is_empty() {
        Damage::Unchanged
    } else {
        Damage::Regions(windows)
    }
}

fn cell_range(
    rect: Rect,
    origin: Point,
    (width, height): (f32, f32),
) -> (Range<usize>, Range<usize>) {
    let x0 = to_cell((rect.origin.x - origin.x) / width);
    let y0 = to_cell((rect.origin.y - origin.y) / height);
    (
        x0..x0.max(to_cell((rect.origin.x + rect.width - origin.x) / width)),
        y0..y0.max(to_cell((rect.origin.y + rect.height - origin.y) / height)),
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "snapped cell edges are whole, non-negative cell counts"
)]
const fn to_cell(value: f32) -> usize {
    if value.is_sign_negative() {
        0
    } else {
        value.round() as usize
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "cell counts stay far below the f32 mantissa"
)]
const fn to_f32(value: usize) -> f32 {
    value as f32
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
