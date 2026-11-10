use std::collections::BTreeSet;
use std::rc::Rc;

use super::patch::{LineId, Patch};
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::style::{Paint, Token};
use crate::widgets::Widget;

/// The diff lines selected in a patch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The hunks.
    pub hunks: BTreeSet<usize>,
    /// The lines.
    pub lines: BTreeSet<LineId>,
    anchor: Option<LineId>,
}

impl Selection {
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

    /// Selects a line and extends the current selection when `extend` is true.
    pub fn select_line(&mut self, patch: &Patch, id: LineId, extend: bool) {
        self.hunks.clear();
        if let Some(anchor) = self.anchor.filter(|_| extend) {
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
}

#[derive(Clone, Copy)]
pub(super) struct HunkPicker {
    pub(super) selection: Signal<Selection>,
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
            |paint| paint.background(Token::Accent3),
        )
    }

    pub(super) fn line(self, row: NodeBuilder, id: LineId) -> NodeBuilder {
        let Self { selection, patch } = self;
        row.pointer_down(move |event| {
            let extend = event.modifiers.shift();
            patch.with(|patch| selection.update(|picked| picked.select_line(patch, id, extend)));
        })
        .paint_when(
            move || selection.with(|picked| picked.contains(id)),
            |paint| paint.background(Token::Accent3),
        )
    }
}
