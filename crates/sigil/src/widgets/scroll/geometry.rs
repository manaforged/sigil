use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use crate::style::intern::intern;
use crate::style::{Metrics, StyleAtom};

use super::{SCROLLBAR_WIDTH, THUMB_MIN_HEIGHT, THUMB_REST_WIDTH, to_i16};

const CACHE_CAP: usize = 256;

pub(crate) type ThumbCache = OnceLock<Mutex<HashMap<(i16, i16), &'static [StyleAtom]>>>;

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
    static CACHE: ThumbCache = OnceLock::new();
    atoms(&CACHE, CACHE_CAP, (0, height), || {
        vec![StyleAtom::Height(crate::style::atom::ATOM_PX, height)]
    })
}

pub(super) fn thumb_atoms(height: i16, thick: bool) -> &'static [StyleAtom] {
    static THIN: ThumbCache = OnceLock::new();
    static THICK: ThumbCache = OnceLock::new();
    let (cache, width) = if thick || Metrics::current().is_grid() {
        (&THICK, SCROLLBAR_WIDTH)
    } else {
        (&THIN, THUMB_REST_WIDTH)
    };
    atoms(cache, CACHE_CAP, (height, 0), || {
        vec![
            StyleAtom::Height(crate::style::atom::ATOM_PX, height),
            StyleAtom::Width(crate::style::atom::ATOM_PX, to_i16(width)),
            StyleAtom::MarginLeft(to_i16((SCROLLBAR_WIDTH - width) / 2.0)),
        ]
    })
}

pub(crate) fn track_atoms((offset, extent): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: ThumbCache = OnceLock::new();
    atoms(&CACHE, CACHE_CAP, (offset, extent), || {
        vec![
            StyleAtom::MarginLeft(offset.wrapping_neg()),
            StyleAtom::MinWidth(crate::style::atom::ATOM_PX, extent),
        ]
    })
}

pub(crate) fn thumb_atoms_x((width, offset): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: ThumbCache = OnceLock::new();
    atoms(&CACHE, CACHE_CAP, (width, offset), || {
        vec![
            StyleAtom::Width(crate::style::atom::ATOM_PX, width),
            StyleAtom::MarginLeft(offset),
        ]
    })
}

pub(crate) fn atoms(
    cache: &'static ThumbCache,
    cap: usize,
    key: (i16, i16),
    build: impl FnOnce() -> Vec<StyleAtom>,
) -> &'static [StyleAtom] {
    let mut map = cache
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let atoms = intern(build());
    if map.len() < cap {
        map.insert(key, atoms);
    }
    atoms
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
