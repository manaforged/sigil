use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Mutex, OnceLock, PoisonError};

use crate::style::StyleAtom;

type AtomCache = OnceLock<Mutex<HashSet<&'static [StyleAtom]>>>;

pub(crate) const KEYED_CAP: usize = 512;

pub(crate) type Keyed<K> = OnceLock<Mutex<HashMap<K, &'static [StyleAtom]>>>;

pub(crate) fn intern(items: impl Into<Box<[StyleAtom]>>) -> &'static [StyleAtom] {
    static CACHE: AtomCache = OnceLock::new();
    let items: Box<[StyleAtom]> = items.into();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(interned) = cache.get(&*items).copied() {
        return interned;
    }
    let interned: &'static [StyleAtom] = Box::leak(items);
    cache.insert(interned);
    interned
}

pub(crate) fn keyed<K: Hash + Eq>(
    cache: &Keyed<K>,
    cap: usize,
    key: K,
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
