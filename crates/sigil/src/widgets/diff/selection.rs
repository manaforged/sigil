use std::collections::BTreeSet;
use std::rc::Rc;

use super::patch::{LineId, Patch};
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::style::{Paint, Tone};
use crate::widgets::Widget;

/// The diff lines selected in a patch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffSelection {
    hunks: BTreeSet<usize>,
    lines: BTreeSet<LineId>,
    anchor: Option<LineId>,
}

impl DiffSelection {
    /// The indexes of the hunks selected as a whole.
    #[must_use]
    pub const fn hunks(&self) -> &BTreeSet<usize> {
        &self.hunks
    }

    /// Returns the ids of the selected lines, in patch order.
    #[must_use]
    pub const fn lines(&self) -> &BTreeSet<LineId> {
        &self.lines
    }

    /// Returns whether `id` is selected.
    #[must_use]
    pub fn contains(&self, id: LineId) -> bool {
        self.lines.contains(&id)
    }

    /// Selects the changed lines of a hunk.
    pub fn select_hunk(&mut self, patch: &Patch, hunk: usize) {
        self.hunks = BTreeSet::from([hunk]);
        self.lines = patch.hunk_changes(hunk).collect();
        self.anchor = None;
    }

    /// Selects a single line.
    pub fn select_line(&mut self, id: LineId) {
        self.hunks.clear();
        self.lines = BTreeSet::from([id]);
        self.anchor = Some(id);
    }

    /// Extends the selection from the anchor to `id`, or selects `id` when no anchor is set.
    pub fn extend_line(&mut self, patch: &Patch, id: LineId) {
        self.hunks.clear();
        if let Some(anchor) = self.anchor {
            let (low, high) = (anchor.min(id), anchor.max(id));
            self.lines = patch
                .changes()
                .filter(|line| (low..=high).contains(line))
                .collect();
        } else {
            self.lines = BTreeSet::from([id]);
            self.anchor = Some(id);
        }
    }

    /// Clears the selection.
    pub fn clear(&mut self) {
        self.hunks.clear();
        self.lines.clear();
        self.anchor = None;
    }
}

#[derive(Clone, Copy)]
pub(super) struct HunkPicker {
    pub(super) selection: Signal<DiffSelection>,
    pub(super) patch: Signal<Rc<Patch>>,
}

impl HunkPicker {
    pub(super) fn hunk(self, row: NodeBuilder, hunk: usize) -> NodeBuilder {
        let Self { selection, patch } = self;
        row.pointer_down(move |_| {
            patch.with(|patch| selection.update(|picked| picked.select_hunk(patch, hunk)));
        })
        .paint_when(
            move || selection.with(|picked| picked.hunks.contains(&hunk)),
            |paint| paint.background(Tone::Primary.step(3)),
        )
    }

    pub(super) fn line(self, row: NodeBuilder, id: LineId) -> NodeBuilder {
        let Self { selection, patch } = self;
        row.pointer_down(move |event| {
            if event.modifiers.shift() {
                patch.with(|patch| selection.update(|picked| picked.extend_line(patch, id)));
            } else {
                selection.update(|picked| picked.select_line(id));
            }
        })
        .paint_when(
            move || selection.with(|picked| picked.contains(id)),
            |paint| paint.background(Tone::Primary.step(3)),
        )
    }
}
