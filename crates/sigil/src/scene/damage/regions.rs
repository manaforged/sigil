use smallvec::SmallVec;

use super::{Damage, FullReason};
use crate::geometry::{Point, Rect};
use crate::scene::geometry::intersect_rect;
use crate::scene::state::snap_out;

const EXACT_MERGE_LIMIT: usize = 24;

type Rects = SmallVec<[Rect; 16]>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Budget {
    pub(crate) max_regions: usize,
    pub(crate) margin: f32,
    pub(crate) max_share: f32,
}

pub(super) fn regions<R: FromIterator<Rect>>(
    inks: &[Rect],
    area: Rect,
    scale: f32,
    budget: Budget,
) -> Damage<R> {
    let rects: Rects = inks
        .iter()
        .filter_map(|ink| intersect_rect(ink.expand(budget.margin), area))
        .map(|rect| snap_out(rect, area.origin, scale))
        .collect();
    if rects.is_empty() {
        return Damage::Unchanged;
    }
    let merged = if rects.len() <= EXACT_MERGE_LIMIT {
        merge_exact(rects, budget.max_regions)
    } else {
        merge_greedy(&rects, budget.max_regions)
    };
    let covered: f32 = merged.iter().map(|rect| rect.width * rect.height).sum();
    if covered > area.width * area.height * budget.max_share {
        return Damage::Full(FullReason::RegionShare);
    }
    Damage::Regions(merged.into_iter().collect())
}

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
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

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
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

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
fn snap_cells(rect: Rect, origin: Point, (width, height): (f32, f32)) -> Rect {
    let across = snap_out(rect, origin, width.recip());
    let down = snap_out(rect, origin, height.recip());
    Rect::new(
        Point::new(across.origin.x, down.origin.y),
        across.width,
        down.height,
    )
}

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
fn absorb(merged: &mut Vec<Rect>, rect: Rect) {
    let mut rect = rect;
    while let Some(index) = merged.iter().position(|region| touches(*region, rect)) {
        rect = rect.union(merged.swap_remove(index));
    }
    merged.push(rect);
}

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
fn touches(a: Rect, b: Rect) -> bool {
    a.origin.x <= b.origin.x + b.width
        && b.origin.x <= a.origin.x + a.width
        && a.origin.y <= b.origin.y + b.height
        && b.origin.y <= a.origin.y + a.height
}

fn merge_exact(mut rects: Rects, cap: usize) -> Rects {
    while rects.len() > 1 {
        let Some((a, b, cost)) = cheapest_merge(&rects) else {
            break;
        };
        if rects.len() <= cap && cost > 0.0 {
            break;
        }
        let removed = rects.swap_remove(b);
        if let Some(kept) = rects.get_mut(a) {
            *kept = kept.union(removed);
        }
    }
    rects
}

fn merge_greedy(rects: &[Rect], cap: usize) -> Rects {
    let mut merged = Rects::new();
    for rect in rects {
        if let Some(home) = merged
            .iter_mut()
            .find(|region| intersect_rect(**region, *rect).is_some())
        {
            *home = home.union(*rect);
        } else if merged.len() < cap {
            merged.push(*rect);
        } else if let Some(home) = merged
            .iter_mut()
            .min_by(|a, b| growth(**a, *rect).total_cmp(&growth(**b, *rect)))
        {
            *home = home.union(*rect);
        }
    }
    merged
}

fn growth(region: Rect, rect: Rect) -> f32 {
    let joined = region.union(rect);
    region
        .width
        .mul_add(-region.height, joined.width * joined.height)
}

fn cheapest_merge(rects: &[Rect]) -> Option<(usize, usize, f32)> {
    let mut best: Option<(usize, usize, f32)> = None;
    for (a, first) in rects.iter().enumerate() {
        for (b, second) in rects.iter().enumerate().skip(a + 1) {
            let cost = second
                .width
                .mul_add(-second.height, growth(*first, *second));
            if best.is_none_or(|(_, _, low)| cost < low) {
                best = Some((a, b, cost));
            }
        }
    }
    best
}
