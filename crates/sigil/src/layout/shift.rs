use std::cell::RefCell;

const INDEX: u32 = (1 << 24) - 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Shift(u32);

#[derive(Clone, Copy, Default)]
struct Entry {
    by: (f32, f32),
    under: Shift,
    era: u8,
}

#[derive(Default)]
struct Slots {
    entries: Vec<Entry>,
    free: Vec<u32>,
}

thread_local! {
    static SLOTS: RefCell<Slots> = const {
        RefCell::new(Slots {
            entries: Vec::new(),
            free: Vec::new(),
        })
    };
}

impl Shift {
    pub(crate) fn offset(self) -> (f32, f32) {
        if self.0 == 0 {
            return (0.0, 0.0);
        }
        SLOTS.with_borrow(|slots| {
            let (mut x, mut y, mut at) = (0.0, 0.0, self);
            for _ in 0..=slots.entries.len() {
                let Some(entry) = slots.entry(at) else {
                    break;
                };
                x += entry.by.0;
                y += entry.by.1;
                at = entry.under;
            }
            (x, y)
        })
    }
}

impl Slots {
    fn at(shift: Shift) -> Option<usize> {
        usize::try_from((shift.0 & INDEX).checked_sub(1)?).ok()
    }

    fn entry(&self, shift: Shift) -> Option<&Entry> {
        self.entries
            .get(Self::at(shift)?)
            .filter(|entry| u32::from(entry.era) == shift.0 >> 24)
    }

    fn entry_mut(&mut self, shift: Shift) -> Option<&mut Entry> {
        self.entries
            .get_mut(Self::at(shift)?)
            .filter(|entry| u32::from(entry.era) == shift.0 >> 24)
    }
}

#[derive(Debug)]
pub(crate) struct ShiftSlot(Shift);

impl ShiftSlot {
    pub(crate) fn new() -> Self {
        SLOTS.with_borrow_mut(|slots| {
            if let Some(index) = slots.free.pop() {
                let era = slots
                    .entries
                    .get(index as usize - 1)
                    .map_or(0, |entry| entry.era);
                return Self(Shift(u32::from(era) << 24 | index));
            }
            slots.entries.push(Entry::default());
            Self(Shift(
                u32::try_from(slots.entries.len())
                    .ok()
                    .filter(|index| *index <= INDEX)
                    .unwrap_or(0),
            ))
        })
    }

    pub(crate) fn move_to(&self, by: (f32, f32)) {
        SLOTS.with_borrow_mut(|slots| {
            if let Some(entry) = slots.entry_mut(self.0) {
                entry.by = by;
            }
        });
    }

    pub(crate) fn nest(&self, under: Shift) -> Shift {
        SLOTS.with_borrow_mut(|slots| {
            if let Some(entry) = slots.entry_mut(self.0)
                && under != self.0
            {
                entry.under = under;
            }
        });
        self.0
    }
}

impl Drop for ShiftSlot {
    fn drop(&mut self) {
        let shift = self.0;
        let _ = SLOTS.try_with(|slots| {
            if let Ok(mut slots) = slots.try_borrow_mut()
                && let Some(entry) = slots.entry_mut(shift)
            {
                *entry = Entry {
                    era: entry.era.wrapping_add(1),
                    ..Entry::default()
                };
                slots.free.push(shift.0 & INDEX);
            }
        });
    }
}
