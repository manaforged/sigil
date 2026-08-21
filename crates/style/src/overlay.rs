//! Per-instance state-slot overlay storage.
//!
//! `StyleOverlay` holds the user-supplied `.hover()` / `.focus()` /
//! `.pressed()` / `.disabled()` atoms for a single widget instance.
//! Unlike widget-baked [`StateStyle`](crate::StateStyle), which stores
//! `&'static [StyleAtom]` slices for slots declared as `static`, an
//! overlay is owned by the widget builder (or its attached
//! `StyleBinding`) — no leaking, no `'static` laundering. The resolver
//! applies an overlay's atoms AFTER the widget's atoms for the same
//! slot, so the render pass's last-atom-wins semantics give user
//! atoms priority per property.

use smallvec::SmallVec;

use crate::{StateSlot, StyleAtom};

/// Dynamic per-state atom slots owned by a single widget instance.
///
/// One of these is built by [`Style::take_state_overlay`](crate::Style::take_state_overlay)
/// and attached to the node's [`StyleBinding`](crate::StyleBinding) at
/// build time. The resolver consults it via [`Self::atoms_for`] to
/// layer user atoms after the widget's own per-slot atoms.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleOverlay {
    rest: SmallVec<[StyleAtom; 4]>,
    hover: SmallVec<[StyleAtom; 4]>,
    focus: SmallVec<[StyleAtom; 4]>,
    pressed: SmallVec<[StyleAtom; 4]>,
    disabled: SmallVec<[StyleAtom; 4]>,
}

impl StyleOverlay {
    /// Build an empty overlay.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an overlay from five per-slot atom vectors.
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

    /// True when no slot has any atoms.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
            && self.hover.is_empty()
            && self.focus.is_empty()
            && self.pressed.is_empty()
            && self.disabled.is_empty()
    }

    /// Borrow the rest-slot atoms.
    #[must_use]
    pub fn rest(&self) -> &[StyleAtom] {
        &self.rest
    }

    /// Borrow the hover-slot atoms.
    #[must_use]
    pub fn hover(&self) -> &[StyleAtom] {
        &self.hover
    }

    /// Borrow the focus-slot atoms.
    #[must_use]
    pub fn focus(&self) -> &[StyleAtom] {
        &self.focus
    }

    /// Borrow the pressed-slot atoms.
    #[must_use]
    pub fn pressed(&self) -> &[StyleAtom] {
        &self.pressed
    }

    /// Borrow the disabled-slot atoms.
    #[must_use]
    pub fn disabled(&self) -> &[StyleAtom] {
        &self.disabled
    }

    /// Atoms for the given slot.
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
