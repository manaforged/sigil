use std::hash::Hash;

use ahash::AHashMap;

/// A map that holds at most `CAP` entries and clears itself when a new key would pass that.
pub(crate) struct Bounded<K, V, const CAP: usize> {
    map: AHashMap<K, V>,
}

impl<K, V, const CAP: usize> Default for Bounded<K, V, CAP> {
    fn default() -> Self {
        Self {
            map: AHashMap::new(),
        }
    }
}

impl<K: Hash + Eq, V, const CAP: usize> Bounded<K, V, CAP> {
    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        self.make_room(&key);
        self.map.insert(key, value);
    }

    #[cfg(feature = "terminal")]
    pub(crate) fn get_or_insert_with(&mut self, key: K, make: impl FnOnce() -> V) -> &mut V {
        self.make_room(&key);
        self.map.entry(key).or_insert_with(make)
    }

    #[cfg(vector_paint)]
    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    fn make_room(&mut self, key: &K) {
        if self.map.len() >= CAP && !self.map.contains_key(key) {
            self.map.clear();
        }
    }
}
