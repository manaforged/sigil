use core::marker::PhantomData;

pub(crate) struct ArenaKey<T> {
    index: u32,
    generation: u32,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for ArenaKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ArenaKey<T> {}

impl<T> PartialEq for ArenaKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for ArenaKey<T> {}

impl<T> PartialOrd for ArenaKey<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for ArenaKey<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.index, self.generation).cmp(&(other.index, other.generation))
    }
}

impl<T> core::hash::Hash for ArenaKey<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> core::fmt::Debug for ArenaKey<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Key({}@{})", self.index, self.generation)
    }
}

impl<T> ArenaKey<T> {
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

struct ArenaSlot<T> {
    generation: u32,
    value: Option<T>,
}

pub(crate) struct Arena<T> {
    slots: Vec<ArenaSlot<T>>,
    free: Vec<u32>,
}

impl<T> Arena<T> {
    pub(crate) const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub(crate) fn insert(&mut self, value: T) -> ArenaKey<T> {
        if let Some(index) = self.free.pop() {
            let slot = self
                .slots
                .get_mut(index as usize)
                .expect("invariant: free list only holds indices of live slots");
            slot.value = Some(value);
            return ArenaKey::from_sendable(index, slot.generation);
        }
        self.slots.push(ArenaSlot {
            generation: 0,
            value: Some(value),
        });
        ArenaKey::from_sendable(
            u32::try_from(self.slots.len() - 1).expect("arena overflow"),
            0,
        )
    }

    pub(crate) fn get(&self, key: ArenaKey<T>) -> Option<&T> {
        let slot = self.slots.get(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_ref()
    }

    pub(crate) fn get_mut(&mut self, key: ArenaKey<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_mut()
    }

    pub(crate) fn remove(&mut self, key: ArenaKey<T>) -> Option<T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(key.index);
        Some(value)
    }

    pub(crate) fn contains(&self, key: ArenaKey<T>) -> bool {
        self.get(key).is_some()
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn keys(&self) -> impl Iterator<Item = ArenaKey<T>> + '_ {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value.as_ref().map(|_| ArenaKey {
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
