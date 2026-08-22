use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_style::{Dimension, Px, StyleAtom};

use super::THUMB_MIN_HEIGHT;

const CACHE_CAP: usize = 256;

type Cache = OnceLock<Mutex<HashMap<(i16, i16), &'static [StyleAtom]>>>;

// Axis-agnostic: the table's horizontal scrollbar reuses this with
// widths in place of heights.
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

pub(super) fn thumb_atoms((height, offset): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: Cache = OnceLock::new();
    atoms(&CACHE, (height, offset), || {
        vec![
            StyleAtom::Height(Dimension::Px(Px(height))),
            StyleAtom::MarginTop(Px(offset)),
        ]
    })
}

/// Shift a horizontally scrolled content track left by `offset` and
/// floor its width at the column `extent`. The floor keeps the track's
/// box spanning its overflowing cells — the emit walk culls subtrees
/// whose box misses the clip, and a viewport-wide track would slide
/// out of view once scrolled past one viewport width. Base width stays
/// `fill_width`, so a fitting table stretches exactly as one without
/// horizontal scrolling.
pub(crate) fn track_atoms((offset, extent): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: Cache = OnceLock::new();
    atoms(&CACHE, (offset, extent), || {
        vec![
            StyleAtom::MarginLeft(Px(offset.wrapping_neg())),
            StyleAtom::MinWidth(Dimension::Px(Px(extent))),
        ]
    })
}

/// Horizontal twin of [`thumb_atoms`] for the table's bottom scrollbar.
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
