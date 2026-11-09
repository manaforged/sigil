use crate::style::intern::{Keyed, keyed};
use crate::style::{Metrics, StyleAtom};

use super::{SCROLLBAR_WIDTH, THUMB_MIN_HEIGHT, THUMB_REST_WIDTH, to_i16};

const CACHE_CAP: usize = 256;

type ThumbCache = Keyed<(i16, i16)>;

pub(crate) fn thumb_metrics(extent: f32, viewport_height: f32) -> (f32, f32) {
    let height = viewport_height.max(0.0);
    let content = extent.max(height);
    let ratio = if content > 0.0 {
        (height / content).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let thumb_h = (ratio * height)
        .max(THUMB_MIN_HEIGHT.min(height))
        .min(height);
    (thumb_h, (height - thumb_h).max(0.0))
}

pub(super) fn content_atoms(height: i16) -> &'static [StyleAtom] {
    static CACHE: ThumbCache = ThumbCache::new();
    keyed(&CACHE, CACHE_CAP, (0, height), || {
        vec![StyleAtom::Height(crate::style::atom::ATOM_PX, height)]
    })
}

pub(super) fn thumb_atoms(height: i16, thick: bool) -> &'static [StyleAtom] {
    static THIN: ThumbCache = ThumbCache::new();
    static THICK: ThumbCache = ThumbCache::new();
    let (cache, width) = if thick || Metrics::current().is_grid() {
        (&THICK, SCROLLBAR_WIDTH)
    } else {
        (&THIN, THUMB_REST_WIDTH)
    };
    keyed(cache, CACHE_CAP, (height, 0), || {
        vec![
            StyleAtom::Height(crate::style::atom::ATOM_PX, height),
            StyleAtom::Width(crate::style::atom::ATOM_PX, to_i16(width)),
            StyleAtom::MarginLeft(to_i16((SCROLLBAR_WIDTH - width) / 2.0)),
        ]
    })
}

pub(crate) fn track_atoms((offset, extent): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: ThumbCache = ThumbCache::new();
    keyed(&CACHE, CACHE_CAP, (offset, extent), || {
        vec![
            StyleAtom::MarginLeft(offset.wrapping_neg()),
            StyleAtom::MinWidth(crate::style::atom::ATOM_PX, extent),
        ]
    })
}

pub(crate) fn thumb_atoms_x((width, offset): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: ThumbCache = ThumbCache::new();
    keyed(&CACHE, CACHE_CAP, (width, offset), || {
        vec![
            StyleAtom::Width(crate::style::atom::ATOM_PX, width),
            StyleAtom::MarginLeft(offset),
        ]
    })
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
