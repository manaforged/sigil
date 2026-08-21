//! Margin-collapse helpers for the CSS block solver.
//!
//! The CSS rule we implement here (the subset that block-only flow
//! demands):
//!
//! * **Adjacent siblings** — the larger of the bottom margin of the
//!   previous in-flow sibling and the top margin of the next in-flow
//!   sibling. Negative margins subtract from the positive collapse;
//!   when both are negative, the more-negative one wins.
//! * **Parent-and-first-child** — if a parent has zero block-axis
//!   padding+border on its top edge, its top margin collapses with the
//!   first in-flow child's top margin. The block solver applies this
//!   by passing the *open* top-margin context into the first child's
//!   layout pass.
//!
//! See DR-041 §"P0.2 block layout" for scope. Out: collapse-through
//! (empty boxes whose top + bottom margins also collapse with each
//! other), floats, clearance.

#![allow(clippy::redundant_pub_crate)]

/// CSS-style margin collapse of two adjacent margins.
///
/// * Both positive → max of the two.
/// * Both negative → min (most-negative wins).
/// * Mixed sign → sum (positive contribution + negative contribution).
pub(crate) fn collapse(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

/// Open-bottom margin state threaded through a sibling sequence. The
/// `current` value is the bottom margin of the most recently placed
/// in-flow sibling, ready to collapse with the next sibling's top
/// margin (or be flushed as parent's bottom-margin contribution).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OpenMargin {
    pub(crate) value: f32,
    pub(crate) active: bool,
}

impl OpenMargin {
    /// Idle state with no margin available to collapse. Used when the
    /// parent has separation and the first sibling's top margin must
    /// land at zero, not collapse against anything.
    pub(crate) const fn sealed() -> Self {
        Self {
            value: 0.0,
            active: false,
        }
    }

    /// Fold a new sibling's top margin into the open context. Returns
    /// the gap (in px) the layout pass should insert before the
    /// sibling's border box. When the context is idle (`active == false`)
    /// the sibling's top margin lands raw (no collapse partner).
    pub(crate) fn absorb_top(&mut self, top: f32) -> f32 {
        if self.active {
            let merged = collapse(self.value, top);
            self.value = 0.0;
            self.active = false;
            merged
        } else {
            self.value = 0.0;
            top
        }
    }

    /// Push the sibling's bottom margin into the open context, ready to
    /// collapse with the next sibling.
    pub(crate) const fn emit_bottom(&mut self, bottom: f32) {
        self.value = bottom;
        self.active = true;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]
#[path = "block_collapse_tests.rs"]
mod tests;
