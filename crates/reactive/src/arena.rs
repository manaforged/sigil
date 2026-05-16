//! Generational slot arena.
//!
//! A small stand-in for `slotmap` with no dependencies. Each `Key`
//! carries its slot index plus the slot's generation at insertion
//! time. `remove` bumps the slot's generation so stale keys return
//! `None` on subsequent lookups.

use core::marker::PhantomData;

/// A generational key into an [`Arena<T>`].
///
/// Two keys with the same index but different generations never
/// resolve to the same value. Keys are `Copy` and carry no runtime
/// pointer; they are safe to hold across arena mutations.
pub(crate) struct Key<T> {
    index: u32,
    generation: u32,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Key<T> {}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for Key<T> {}

impl<T> PartialOrd for Key<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Key<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.index, self.generation).cmp(&(other.index, other.generation))
    }
}

impl<T> core::hash::Hash for Key<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> core::fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Key({}@{})", self.index, self.generation)
    }
}

impl<T> Key<T> {
    /// Extract the raw index component for cross-thread identity use.
    ///
    /// Paired with [`Key::generation_for_sendable`] and
    /// [`Key::from_sendable`] to let a specific typed wrapper (today:
    /// `SendableSignal<T>` in `signal.rs`) carry an arena identity
    /// across a thread boundary without exposing the full `Key`.
    pub(crate) const fn index_for_sendable(self) -> u32 {
        self.index
    }

    /// Extract the raw generation component. See
    /// [`Key::index_for_sendable`].
    pub(crate) const fn generation_for_sendable(self) -> u32 {
        self.generation
    }

    /// Re-construct a key from the raw `(index, generation)` produced
    /// by [`Key::index_for_sendable`] and
    /// [`Key::generation_for_sendable`]. See [`Key::index_for_sendable`].
    pub(crate) const fn from_sendable(index: u32, generation: u32) -> Self {
        Self {
            index,
            generation,
            _ty: PhantomData,
        }
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// A generational slot storage.
pub(crate) struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Arena<T> {
    pub(crate) const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub(crate) fn insert(&mut self, value: T) -> Key<T> {
        if let Some(index) = self.free.pop() {
            let slot = self
                .slots
                .get_mut(index as usize)
                .expect("invariant: free list only holds indices of live slots");
            slot.value = Some(value);
            return Key {
                index,
                generation: slot.generation,
                _ty: PhantomData,
            };
        }
        let index = u32::try_from(self.slots.len()).expect("arena overflow");
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Key {
            index,
            generation: 0,
            _ty: PhantomData,
        }
    }

    pub(crate) fn get(&self, key: Key<T>) -> Option<&T> {
        let slot = self.slots.get(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_ref()
    }

    pub(crate) fn get_mut(&mut self, key: Key<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_mut()
    }

    pub(crate) fn remove(&mut self, key: Key<T>) -> Option<T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(key.index);
        Some(value)
    }

    pub(crate) fn contains(&self, key: Key<T>) -> bool {
        self.get(key).is_some()
    }

    /// Iterate over every live key in the arena, in insertion-slot
    /// order. Used by the debug tree dump to locate root owners.
    pub(crate) fn keys(&self) -> impl Iterator<Item = Key<T>> + '_ {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            if slot.value.is_some() {
                Some(Key {
                    index: u32::try_from(index).expect("arena overflow"),
                    generation: slot.generation,
                    _ty: PhantomData,
                })
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_remove_roundtrip() {
        let mut arena = Arena::<u32>::new();
        let a = arena.insert(1);
        let b = arena.insert(2);
        assert_eq!(arena.get(a), Some(&1));
        assert_eq!(arena.get(b), Some(&2));
        assert_eq!(arena.remove(a), Some(1));
        assert_eq!(arena.get(a), None);
        assert!(!arena.contains(a));
    }

    #[test]
    fn stale_key_after_reuse_returns_none() {
        let mut arena = Arena::<&'static str>::new();
        let a = arena.insert("first");
        arena.remove(a);
        let b = arena.insert("second");
        // Same slot index, different generation.
        assert_eq!(arena.get(a), None);
        assert_eq!(arena.get(b), Some(&"second"));
    }
}
