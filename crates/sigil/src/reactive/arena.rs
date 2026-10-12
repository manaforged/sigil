use core::marker::PhantomData;

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
    pub(crate) const fn index_for_sendable(self) -> u32 {
        self.index
    }

    pub(crate) const fn generation_for_sendable(self) -> u32 {
        self.generation
    }

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
            return Key::from_sendable(index, slot.generation);
        }
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Key::from_sendable(
            u32::try_from(self.slots.len() - 1).expect("arena overflow"),
            0,
        )
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

    pub(crate) fn keys(&self) -> impl Iterator<Item = Key<T>> + '_ {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value.as_ref().map(|_| Key {
                index: u32::try_from(index).expect("arena overflow"),
                generation: slot.generation,
                _ty: PhantomData,
            })
        })
    }
}

#[cfg(test)]
#[path = "arena_tests.rs"]
mod tests;
