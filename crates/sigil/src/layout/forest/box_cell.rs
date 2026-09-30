use std::cell::Cell;

use super::ComputedBox;

#[derive(Debug, Default)]
pub(crate) struct BoxCell {
    cb: Cell<ComputedBox>,
    frame: Cell<u32>,
}

impl BoxCell {
    #[must_use]
    pub(crate) const fn new(cb: ComputedBox) -> Self {
        Self {
            cb: Cell::new(cb),
            frame: Cell::new(0),
        }
    }

    pub(crate) const fn get(&self) -> ComputedBox {
        self.cb.get()
    }

    pub(crate) const fn frame(&self) -> u32 {
        self.frame.get()
    }

    pub(crate) fn write(&self, cb: ComputedBox, frame: u32) {
        self.cb.set(cb);
        self.frame.set(frame);
    }

    pub(crate) fn reset(&self) {
        self.cb.set(ComputedBox::default());
        self.frame.set(0);
    }
}
