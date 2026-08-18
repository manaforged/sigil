//! Shared table chrome: context-menu target and row-menu callback.

use std::rc::Rc;

use sigil_reactive::{Signal, signal};

use super::super::menu::MenuItem;

/// What the open context menu is pointed at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContextKind {
    Header(&'static str),
    Row(u32),
}

pub(super) type RowMenu<Row> = Rc<dyn Fn(&Row) -> Vec<MenuItem>>;

/// Per-table overlay state. Cheap to clone (signals are `Copy`).
pub(super) struct TableChrome<Row: 'static> {
    pub open: Signal<bool>,
    pub pos: Signal<(f32, f32)>,
    pub kind: Signal<Option<ContextKind>>,
    pub row_menu: Option<RowMenu<Row>>,
}

impl<Row: 'static> Clone for TableChrome<Row> {
    fn clone(&self) -> Self {
        Self {
            open: self.open,
            pos: self.pos,
            kind: self.kind,
            row_menu: self.row_menu.clone(),
        }
    }
}

impl<Row: 'static> TableChrome<Row> {
    pub(super) fn new(row_menu: Option<RowMenu<Row>>) -> Self {
        Self {
            open: signal(false),
            pos: signal((0.0, 0.0)),
            kind: signal(None),
            row_menu,
        }
    }

    pub(super) fn open(&self, kind: ContextKind, pos: impl Into<(f32, f32)>) {
        self.kind.set(Some(kind));
        self.pos.set(pos.into());
        self.open.set(true);
    }
}
