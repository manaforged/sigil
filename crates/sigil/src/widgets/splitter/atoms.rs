use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::style::{Dimension as StyleDimension, Px, StyleAtom};

const ATOM_CACHE_CAP: usize = 512;

pub(in crate::widgets) fn cached_width_atom(px: i16) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();

    cached_atom(&CACHE, px, |p| {
        vec![StyleAtom::Width(StyleDimension::Px(Px(p)))]
    })
}

pub(super) fn cached_height_atom(px: i16) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();

    cached_atom(&CACHE, px, |p| {
        vec![StyleAtom::Height(StyleDimension::Px(Px(p)))]
    })
}

fn cached_atom(
    cache: &'static OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>>,

    key: i16,

    build: impl FnOnce(i16) -> Vec<StyleAtom>,
) -> &'static [StyleAtom] {
    let map = cache.get_or_init(|| Mutex::new(HashMap::new()));

    let Ok(mut guard) = map.lock() else {
        return Box::leak(build(key).into_boxed_slice());
    };

    if let Some(atoms) = guard.get(&key).copied() {
        return atoms;
    }

    let leaked: &'static [StyleAtom] = Box::leak(build(key).into_boxed_slice());

    if guard.len() < ATOM_CACHE_CAP {
        guard.insert(key, leaked);
    }

    leaked
}

pub(in crate::widgets) fn to_i16(px: f32) -> i16 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let clamped = px.clamp(f32::from(i16::MIN), f32::from(i16::MAX)).round() as i16;

    clamped
}
