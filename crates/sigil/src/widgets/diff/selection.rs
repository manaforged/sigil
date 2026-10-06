use std::collections::BTreeSet;
use std::rc::Rc;

use super::patch::{LineId, Patch};
use crate::events::NodeEventsExt;
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::style::{NodeStyleExt, Paint, Token};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub hunks: BTreeSet<usize>,
    pub lines: BTreeSet<LineId>,
    anchor: Option<LineId>,
}

impl Selection {
    #[must_use]
    pub fn contains(&self, id: LineId) -> bool {
        self.lines.contains(&id)
    }

    pub fn select_hunk(&mut self, patch: &Patch, hunk: usize) {
        self.hunks = BTreeSet::from([hunk]);
        self.lines = patch.hunk_changes(hunk).collect();
        self.anchor = None;
    }

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
pub(super) struct Picker {
    pub(super) selection: Signal<Selection>,
    pub(super) patch: Signal<Rc<Patch>>,
}

impl Picker {
    pub(super) fn hunk(self, row: NodeBuilder, hunk: usize) -> NodeBuilder {
        let Self { selection, patch } = self;
        row.pointer_down(move |_| {
            patch.with(|patch| selection.update(|picked| picked.select_hunk(patch, hunk)));
        })
        .paint_when(
            move || selection.with(|picked| picked.hunks.contains(&hunk)),
            |paint| paint.bg(Token::Accent3),
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
            |paint| paint.bg(Token::Accent3),
        )
    }
}
