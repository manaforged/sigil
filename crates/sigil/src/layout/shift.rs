use std::cell::RefCell;

const HOPS: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Shift(u32);

#[derive(Clone, Copy, Default)]
struct Entry {
    by: (f32, f32),
    under: Shift,
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
            for _ in 0..HOPS {
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
    fn entry(&self, shift: Shift) -> Option<&Entry> {
        self.entries.get(usize::try_from(shift.0.checked_sub(1)?).ok()?)
    }

    fn entry_mut(&mut self, shift: Shift) -> Option<&mut Entry> {
        self.entries
            .get_mut(usize::try_from(shift.0.checked_sub(1)?).ok()?)
    }
}

#[derive(Debug)]
pub(crate) struct ShiftSlot(Shift);

impl ShiftSlot {
    pub(crate) fn new() -> Self {
        SLOTS.with_borrow_mut(|slots| {
            if let Some(index) = slots.free.pop() {
                return Self(Shift(index));
            }
            slots.entries.push(Entry::default());
            Self(Shift(u32::try_from(slots.entries.len()).unwrap_or(0)))
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
            if let Ok(mut slots) = slots.try_borrow_mut() {
                if let Some(entry) = slots.entry_mut(shift) {
                    *entry = Entry::default();
                }
                if shift.0 != 0 {
                    slots.free.push(shift.0);
                }
            }
        });
    }
}
