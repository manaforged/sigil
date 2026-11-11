use std::collections::HashSet;
use std::hash::Hash;
use std::sync::{Mutex, OnceLock, PoisonError};

/// The keys a diagnostic has already reported, so each one fires once.
pub(crate) struct Seen<K>(OnceLock<Mutex<HashSet<K>>>);

impl<K: Eq + Hash> Seen<K> {
    /// Creates an empty set.
    pub(crate) const fn new() -> Self {
        Self(OnceLock::new())
    }

    /// Records `key` and returns whether it was not seen before.
    pub(crate) fn first(&self, key: K) -> bool {
        self.0
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key)
    }

    /// Counts the recorded keys that match `keep`.
    #[cfg(test)]
    pub(crate) fn count_where(&self, keep: impl Fn(&K) -> bool) -> usize {
        self.0.get().map_or(0, |seen| {
            seen.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .filter(|key| keep(key))
                .count()
        })
    }
}
