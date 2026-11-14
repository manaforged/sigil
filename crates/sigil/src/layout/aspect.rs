#![allow(clippy::redundant_pub_crate)]

use crate::layout::block_size::SizeIntent;
use crate::layout::resolve::NodeEdges;
use crate::layout::snap;
use crate::layout::style_data::{Axes, BoxSizing, Length, StyleData};

pub(crate) fn derive(
    style: &StyleData,
    size: f32,
    edges: &NodeEdges,
    to_height: bool,
) -> Option<f32> {
    let ratio = style.aspect_ratio?;
    let (from, to, factor) = if to_height {
        (edges.inline_chrome(), edges.block_chrome(), ratio.recip())
    } else {
        (edges.block_chrome(), edges.inline_chrome(), ratio)
    };
    let derived = match style.box_sizing {
        BoxSizing::BorderBox => size * factor,
        BoxSizing::ContentBox => (size - from).max(0.0).mul_add(factor, to),
    };
    Some(snap::span(derived.max(0.0)))
}

pub(crate) const fn sizes_height(style: &StyleData) -> bool {
    style.aspect_ratio.is_some() && matches!(style.size.height, Length::Auto)
}

pub(crate) fn assign(
    style: &StyleData,
    edges: &NodeEdges,
    cb: Axes<f32>,
    (width, height): (Option<f32>, Option<f32>),
) -> (Option<f32>, Option<f32>) {
    if style.aspect_ratio.is_none() || (width.is_some() && height.is_some()) {
        return (width, height);
    }
    let intent = SizeIntent::build(style, cb, edges);
    let width = width.or_else(|| intent.w.preferred.map(|w| intent.w.clamp(w)));
    let height = height.or_else(|| intent.h.preferred.map(|h| intent.h.clamp(h)));
    match (width, height) {
        (Some(w), None) => (
            width,
            derive(style, w, edges, true).map(|to| intent.h.clamp(to)),
        ),
        (None, Some(h)) => (
            derive(style, h, edges, false).map(|to| intent.w.clamp(to)),
            height,
        ),
        known => known,
    }
}
