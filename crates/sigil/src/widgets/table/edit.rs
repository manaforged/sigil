use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use crate::events::pointer_dispatch::Clicks;
use crate::events::{Key, KeyEvent, MouseButton, NamedKey, PointerEvent};
use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::reactive::{Signal, memo, signal};
use crate::widgets::{Layout, Widget};

use super::super::text_field::text_field;
use super::columns::TableColumn;
use super::state::TableState;

type CellText<Row> = Rc<dyn Fn(&Row) -> String>;
type CellCommit<Row> = Rc<dyn Fn(&[Row], usize, String) -> Option<Vec<Row>>>;

pub(super) struct ColumnEdit<Row> {
    text: CellText<Row>,
    commit: CellCommit<Row>,
}

impl<Row> Clone for ColumnEdit<Row> {
    fn clone(&self) -> Self {
        Self {
            text: Rc::clone(&self.text),
            commit: Rc::clone(&self.commit),
        }
    }
}

impl<Row: Clone + 'static> ColumnEdit<Row> {
    pub(super) fn new(
        text: impl Fn(&Row) -> String + 'static,
        commit: impl Fn(&mut Row, String) + 'static,
    ) -> Self {
        Self {
            text: Rc::new(text),
            commit: Rc::new(move |rows, index, value| {
                let mut next = rows.to_vec();
                commit(next.get_mut(index)?, value);
                Some(next)
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CellRef {
    pub(super) row: u32,
    pub(super) column: usize,
}

#[derive(Clone, Copy)]
pub(super) struct CellEditor {
    buffer: Signal<String>,
    pub(super) follow: Signal<usize>,
    refocus: Signal<(u32, u64)>,
    clicks: Signal<Clicks<CellRef>>,
}

impl CellEditor {
    pub(super) fn new() -> Self {
        Self {
            buffer: signal(String::new()),
            follow: signal(0),
            refocus: signal((0, 0)),
            clicks: signal(Clicks::default()),
        }
    }

    fn start<Row: 'static>(self, state: &TableState<Row>, cell: CellRef) {
        self.commit(state);
        let rows = state.rows().peek();
        let (Some(row), Some(edit)) = (
            rows.get(cell.row as usize),
            state
                .columns()
                .get(cell.column)
                .and_then(TableColumn::cell_edit),
        ) else {
            return;
        };
        self.buffer.set((edit.text)(row));
        if let Some(at) = state.visible().peek().iter().position(|&r| r == cell.row) {
            self.follow.set(at);
        }
        state.editing().set(Some(cell));
    }

    fn commit<Row: 'static>(self, state: &TableState<Row>) {
        let Some(cell) = state.editing().peek() else {
            return;
        };
        let Some(edit) = state
            .columns()
            .get(cell.column)
            .and_then(TableColumn::cell_edit)
        else {
            return;
        };
        let rows = state.rows().peek();
        let index = cell.row as usize;
        let value = self.buffer.peek();
        if rows.get(index).is_some_and(|row| (edit.text)(row) != value)
            && let Some(next) = (edit.commit)(&rows, index, value)
        {
            state.replace_rows(Arc::new(next));
        }
    }

    fn leave<Row: 'static>(self, state: &TableState<Row>, cell: CellRef) {
        if state.editing().peek() == Some(cell) {
            self.commit(state);
            state.editing().set(None);
        }
    }

    fn finish<Row: 'static>(self, state: &TableState<Row>, row: u32) {
        state.editing().set(None);
        let (_, generation) = self.refocus.peek();
        self.refocus.set((row, generation.wrapping_add(1)));
    }

    fn step<Row: 'static>(self, state: &TableState<Row>, cell: CellRef, forward: bool) {
        let order = editable_cells(state);
        let next = order.iter().position(|&at| at == cell).and_then(|at| {
            if forward {
                order.get(at.saturating_add(1))
            } else {
                at.checked_sub(1).and_then(|back| order.get(back))
            }
        });
        if let Some(&next) = next {
            self.start(state, next);
        } else {
            self.commit(state);
            self.finish(state, cell.row);
        }
    }

    pub(super) fn begin_row<Row: 'static>(self, state: &TableState<Row>, row: u32) -> bool {
        let first = state
            .visible_columns()
            .into_iter()
            .find(|(_, column)| column.is_editable());
        first.is_some_and(|(column, _)| {
            self.start(state, CellRef { row, column });
            true
        })
    }

    pub(super) fn refocus_trigger(self, row: u32) -> Signal<u64> {
        let refocus = self.refocus;
        let mine = Cell::new(0_u64);
        memo(move || {
            let (target, generation) = refocus.get();
            if target == row {
                mine.set(generation);
            }
            mine.get()
        })
    }
}

fn editable_cells<Row: 'static>(state: &TableState<Row>) -> Vec<CellRef> {
    let columns: Vec<usize> = state
        .visible_columns()
        .into_iter()
        .filter(|(_, column)| column.is_editable())
        .map(|(index, _)| index)
        .collect();
    state
        .visible()
        .peek()
        .iter()
        .flat_map(|&row| columns.iter().map(move |&column| CellRef { row, column }))
        .collect()
}

pub(super) fn editable_cell<Row: 'static>(
    state: &TableState<Row>,
    editor: CellEditor,
    (row, column): (u32, usize),
    disabled: Option<Signal<bool>>,
    content: impl Fn() -> NodeBuilder + 'static,
) -> NodeBuilder {
    let cell = CellRef { row, column };
    let target = state.editing();
    let editing = memo(move || target.get() == Some(cell));
    let title = state
        .columns()
        .get(column)
        .map(|col| col.title().to_owned())
        .unwrap_or_default();
    let keys = state.clone();
    let clicks = state.clone();
    container()
        .layout(LayoutStyle::flex_row().fill_width())
        .children_reactive(move || {
            if editing.get() {
                vec![editor_field(&keys, editor, cell, &title)]
            } else {
                vec![content()]
            }
        })
        .pointer_down(move |event| {
            if !disabled.is_some_and(|off| off.peek()) {
                double_click(&clicks, editor, cell, event);
            }
        })
}

fn editor_field<Row: 'static>(
    state: &TableState<Row>,
    editor: CellEditor,
    cell: CellRef,
    title: &str,
) -> NodeBuilder {
    let keys = state.clone();
    let blurred = state.clone();
    container()
        .layout(LayoutStyle::flex_row().fill_width())
        .key_capture_any(move |event| edit_key(&keys, editor, cell, event))
        .child(
            text_field(editor.buffer)
                .placeholder(title.to_owned())
                .blur(move || editor.leave(&blurred, cell))
                .autofocus(),
        )
}

fn edit_key<Row: 'static>(
    state: &TableState<Row>,
    editor: CellEditor,
    cell: CellRef,
    event: &mut KeyEvent,
) {
    let Key::Named(key) = &event.key else {
        return;
    };
    match *key {
        NamedKey::Enter => {
            editor.commit(state);
            editor.finish(state, cell.row);
        }
        NamedKey::Escape => editor.finish(state, cell.row),
        NamedKey::Tab => editor.step(state, cell, !event.modifiers.shift()),
        _ => return,
    }
    event.stop_propagation();
}

fn double_click<Row: 'static>(
    state: &TableState<Row>,
    editor: CellEditor,
    cell: CellRef,
    event: &mut PointerEvent,
) {
    if event.button != Some(MouseButton::Left) || state.editing().peek() == Some(cell) {
        return;
    }
    let mut clicks = editor.clicks.peek();
    let count = clicks.press(cell, event.window_position);
    editor.clicks.set(clicks);
    if count == 2 {
        editor.start(state, cell);
        event.stop_propagation();
    }
}
