use smallvec::SmallVec;

use super::{AA_MARGIN, MAX_REGION_SHARE, MAX_REGIONS, Plan, Regions};
use crate::geometry::Rect;
use crate::renderer::scene_command::state::snap_out;
use crate::renderer::scene_command::walk::intersect_rect;

const EXACT_MERGE_LIMIT: usize = 24;

pub(super) fn regions(inks: &[Rect], area: Rect, scale: f32) -> Plan {
    let rects: SmallVec<[Rect; 16]> = inks
        .iter()
        .filter_map(|ink| intersect_rect(ink.expand(AA_MARGIN), area))
        .map(|rect| snap_out(rect, area.origin, scale))
        .collect();
    if rects.is_empty() {
        return Plan::Unchanged;
    }
    let merged = if rects.len() <= EXACT_MERGE_LIMIT {
        merge_exact(rects)
    } else {
        merge_greedy(&rects)
    };
    let covered: f32 = merged.iter().map(|rect| rect.width * rect.height).sum();
    if covered > area.width * area.height * MAX_REGION_SHARE {
        return Plan::Full;
    }
    Plan::Regions(merged)
}

fn merge_exact(mut rects: SmallVec<[Rect; 16]>) -> Regions {
    while rects.len() > 1 {
        let Some((a, b, cost)) = cheapest_merge(&rects) else {
            break;
        };
        if rects.len() <= MAX_REGIONS && cost > 0.0 {
            break;
        }
        let removed = rects.swap_remove(b);
        if let Some(kept) = rects.get_mut(a) {
            *kept = kept.union(removed);
        }
    }
    rects.into_iter().collect()
}

fn merge_greedy(rects: &[Rect]) -> Regions {
    let mut merged: Regions = SmallVec::new();
    for rect in rects {
        if let Some(home) = merged
            .iter_mut()
            .find(|region| intersect_rect(**region, *rect).is_some())
        {
            *home = home.union(*rect);
        } else if merged.len() < MAX_REGIONS {
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
