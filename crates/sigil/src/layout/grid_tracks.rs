#![allow(clippy::indexing_slicing, clippy::cast_precision_loss)]

use crate::layout::forest::SpaceConstraint;
use crate::layout::style_data::{Dimension, LengthPercentage, StyleData, TrackSize};

pub(super) fn grid_box_h(style: &StyleData, avail_h: f32, content_h: f32, chrome: f32) -> f32 {
    match style.size.height {
        Dimension::Px(px) => px,
        Dimension::Text(length) => length.px(),
        Dimension::Fill => avail_h,
        Dimension::Percent(pct) => avail_h * pct,
        Dimension::Auto if avail_h > 0.0 => avail_h,
        Dimension::Auto => content_h + chrome,
    }
}

pub(super) fn spanned_height(row_h: &[f32], row: usize, span: usize, gap: f32) -> f32 {
    let end = (row + span).min(row_h.len());
    if row >= end {
        return 0.0;
    }
    let tracks = &row_h[row..end];
    gap.mul_add(
        (tracks.len().saturating_sub(1)) as f32,
        tracks.iter().sum::<f32>(),
    )
}

pub(super) fn offsets(start: f32, sizes: &[f32], gap: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(sizes.len());
    let mut pos = start;
    for size in sizes {
        out.push(pos);
        pos += size + gap;
    }
    out
}

pub(super) fn resolve_columns(tracks: &[TrackSize], inner_w: f32, gap_x: f32) -> Vec<f32> {
    if tracks.is_empty() {
        return vec![inner_w.max(0.0)];
    }
    let total_gap = gap_x * (tracks.len().saturating_sub(1) as f32);
    let mut sizes: Vec<Option<f32>> = tracks
        .iter()
        .map(|t| match t {
            TrackSize::Px(px) => Some(px.max(0.0)),
            _ => None,
        })
        .collect();
    loop {
        let fixed: f32 = sizes.iter().flatten().sum();
        let free = (inner_w - total_gap - fixed).max(0.0);
        let active_weight: f32 = tracks
            .iter()
            .zip(&sizes)
            .filter(|(_, s)| s.is_none())
            .map(|(t, _)| flex_weight(t))
            .sum();
        let per_fr = if active_weight > 0.0 {
            free / active_weight
        } else {
            0.0
        };
        let mut froze = false;
        for (t, s) in tracks.iter().zip(sizes.iter_mut()) {
            if s.is_none()
                && let TrackSize::MinMax(min, fr) = t
                && per_fr * fr.max(0.0) < min.max(0.0)
            {
                *s = Some(min.max(0.0));
                froze = true;
                break;
            }
        }
        if froze {
            continue;
        }
        for (t, s) in tracks.iter().zip(sizes.iter_mut()) {
            if s.is_none() {
                *s = Some(per_fr * flex_weight(t));
            }
        }
        break;
    }
    sizes.into_iter().map(|s| s.unwrap_or(0.0)).collect()
}

pub(super) const fn flex_weight(track: &TrackSize) -> f32 {
    match track {
        TrackSize::Fr(fr) | TrackSize::MinMax(_, fr) => fr.max(0.0),
        TrackSize::Auto => 1.0,
        TrackSize::Px(_) => 0.0,
    }
}

pub(super) fn distribute_fr_rows(row_h: &mut [f32], tracks: &[TrackSize], inner_h: f32, gap: f32) {
    if row_h.is_empty() {
        return;
    }
    let gaps = gap * (row_h.len().saturating_sub(1) as f32);
    let used: f32 = row_h.iter().sum::<f32>() + gaps;
    let leftover = inner_h - used;
    if leftover <= 0.0 {
        return;
    }
    let weight: f32 = (0..row_h.len())
        .map(|index| tracks.get(index).map_or(0.0, flex_weight))
        .sum();
    if weight <= 0.0 {
        return;
    }
    for (index, height) in row_h.iter_mut().enumerate() {
        let share = tracks.get(index).map_or(0.0, flex_weight);
        if share > 0.0 {
            *height = leftover.mul_add(share / weight, *height);
        }
    }
}

pub(super) fn apply_row_template(row_h: &mut [f32], tracks: &[TrackSize]) {
    for (row, h) in row_h.iter_mut().enumerate() {
        if let Some(TrackSize::Px(px)) = tracks.get(row) {
            *h = px.max(0.0);
        }
    }
}

pub(super) fn resolve_gap(gap: LengthPercentage, inner_w: f32, snap: fn(f32) -> f32) -> f32 {
    let resolved = match gap {
        LengthPercentage::Px(px) => px.max(0.0),
        LengthPercentage::Percent(pct) => (pct * inner_w).max(0.0),
        LengthPercentage::Text(length) => length.px().max(0.0),
    };
    snap(resolved)
}

pub(super) const fn definite(c: SpaceConstraint) -> f32 {
    match c {
        SpaceConstraint::Definite(v) => v,
        SpaceConstraint::MinContent | SpaceConstraint::MaxContent => 0.0,
    }
}
