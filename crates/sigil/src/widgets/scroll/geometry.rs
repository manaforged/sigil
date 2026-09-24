use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::style::{Dimension, Px, StyleAtom};

use super::{SCROLLBAR_WIDTH, THUMB_MIN_HEIGHT, THUMB_REST_WIDTH, to_i16};

const CACHE_CAP: usize = 256;

type Cache = OnceLock<Mutex<HashMap<(i16, i16), &'static [StyleAtom]>>>;

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

pub(super) fn content_atoms((offset, height): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: Cache = OnceLock::new();
    atoms(&CACHE, (offset, height), || {
        vec![
            StyleAtom::MarginTop(Px(offset.wrapping_neg())),
            StyleAtom::Height(Dimension::Px(Px(height))),
        ]
    })
}

pub(super) fn thumb_atoms((height, offset): (i16, i16), thick: bool) -> &'static [StyleAtom] {
    static THIN: Cache = OnceLock::new();
    static THICK: Cache = OnceLock::new();
    let (cache, width) = if thick {
        (&THICK, SCROLLBAR_WIDTH)
    } else {
        (&THIN, THUMB_REST_WIDTH)
    };
    atoms(cache, (height, offset), || {
        vec![
            StyleAtom::Height(Dimension::Px(Px(height))),
            StyleAtom::MarginTop(Px(offset)),
            StyleAtom::Width(Dimension::Px(Px(to_i16(width)))),
            StyleAtom::MarginLeft(Px(to_i16((SCROLLBAR_WIDTH - width) / 2.0))),
        ]
    })
}

pub(crate) fn track_atoms((offset, extent): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: Cache = OnceLock::new();
    atoms(&CACHE, (offset, extent), || {
        vec![
            StyleAtom::MarginLeft(Px(offset.wrapping_neg())),
            StyleAtom::MinWidth(Dimension::Px(Px(extent))),
        ]
    })
}

pub(crate) fn thumb_atoms_x((width, offset): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: Cache = OnceLock::new();
    atoms(&CACHE, (width, offset), || {
        vec![
            StyleAtom::Width(Dimension::Px(Px(width))),
            StyleAtom::MarginLeft(Px(offset)),
        ]
    })
}

fn atoms(
    cache: &'static Cache,
    key: (i16, i16),
    build: impl FnOnce() -> Vec<StyleAtom>,
) -> &'static [StyleAtom] {
    let cache = cache.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = cache.lock() else {
        return Box::leak(build().into_boxed_slice());
    };
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let leaked = Box::leak(build().into_boxed_slice());
    if map.len() < CACHE_CAP {
        map.insert(key, leaked);
    }
    leaked
}
