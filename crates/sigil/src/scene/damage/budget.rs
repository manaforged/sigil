use smallvec::SmallVec;

use super::{Damage, FullReason};
use crate::geometry::Rect;
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
