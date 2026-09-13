use smallvec::SmallVec;

use crate::style::{StateSlot, StyleAtom};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleOverlay {
    rest: SmallVec<[StyleAtom; 4]>,
    hover: SmallVec<[StyleAtom; 4]>,
    focus: SmallVec<[StyleAtom; 4]>,
    pressed: SmallVec<[StyleAtom; 4]>,
    disabled: SmallVec<[StyleAtom; 4]>,
}

impl StyleOverlay {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn from_slots(
        rest: SmallVec<[StyleAtom; 4]>,
        hover: SmallVec<[StyleAtom; 4]>,
        focus: SmallVec<[StyleAtom; 4]>,
        pressed: SmallVec<[StyleAtom; 4]>,
        disabled: SmallVec<[StyleAtom; 4]>,
    ) -> Self {
        Self {
            rest,
            hover,
            focus,
            pressed,
            disabled,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
            && self.hover.is_empty()
            && self.focus.is_empty()
            && self.pressed.is_empty()
            && self.disabled.is_empty()
    }

    #[must_use]
    pub fn rest(&self) -> &[StyleAtom] {
        &self.rest
    }

    #[must_use]
    pub fn hover(&self) -> &[StyleAtom] {
        &self.hover
    }

    #[must_use]
    pub fn focus(&self) -> &[StyleAtom] {
        &self.focus
    }

    #[must_use]
    pub fn pressed(&self) -> &[StyleAtom] {
        &self.pressed
    }

    #[must_use]
    pub fn disabled(&self) -> &[StyleAtom] {
        &self.disabled
    }

    #[must_use]
    pub fn atoms_for(&self, slot: StateSlot) -> &[StyleAtom] {
        match slot {
            StateSlot::Rest => &self.rest,
            StateSlot::Focused => &self.focus,
            StateSlot::Hovered => &self.hover,
            StateSlot::Pressed => &self.pressed,
            StateSlot::Disabled => &self.disabled,
        }
    }
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
