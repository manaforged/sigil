use std::cmp::Ordering;
use std::rc::Rc;

use data_table::{FIELDS, Field, FieldKind, fields::header_label};
use sigil::prelude::*;
use sigil::reactive::derived_eq;

use crate::cells::cell;
use data_table::ledger::{Row, UNDO_DEPTH, compare, generate, parse_price, prices, tick};

#[derive(Clone, Copy)]
pub struct Live {
    pub prices: Signal<Rc<Vec<f64>>>,
    pub editing: Signal<Option<u32>>,
    pub draft: Signal<String>,
    pub focus: Signal<u64>,
    pub undo: Signal<Vec<(u32, f64)>>,
    pub ticks: Signal<u64>,
    pub moved: Signal<u64>,
}

#[derive(Clone)]
pub struct App {
    pub table: TableState<Row>,
    pub live: Live,
    pub dark: Signal<bool>,
    pub scroll: Signal<f32>,
}

impl App {
    #[must_use]
    pub fn new(rows: usize) -> Self {
        let book = generate(rows);
        let live = Live::new(prices(&book));
        Self {
            table: TableState::new(book, columns(live)),
            live,
            dark: signal(false),
            scroll: signal(0.0),
        }
    }

    pub fn edit_selected(&self) {
        if let Some(row) = self.table.selected_index() {
            self.live.edit(row);
        }
    }

    pub fn toggle_theme(&self) {
        self.dark.update(|dark| *dark = !*dark);
    }
}

impl Live {
    #[must_use]
    pub fn new(prices: Vec<f64>) -> Self {
        Self {
            prices: signal(Rc::new(prices)),
            editing: signal(None),
            draft: signal(String::new()),
            focus: signal(0),
            undo: signal(Vec::new()),
            ticks: signal(0),
            moved: signal(0),
        }
    }

    pub fn tick(&self) {
        let step = self.ticks.peek();
        let mut moved = 0;
        self.prices.update(|prices| {
            moved = tick(Rc::make_mut(prices).as_mut_slice(), step);
        });
        self.ticks.set(step + 1);
        let moved = u64::try_from(moved).unwrap_or(0);
        self.moved.update(|total| *total += moved);
    }

    #[must_use]
    pub fn price(&self, row: u32) -> Option<f64> {
        self.prices.peek().get(usize::try_from(row).ok()?).copied()
    }

    pub fn edit(&self, row: u32) {
        let Some(current) = self.price(row) else {
            return;
        };
        self.draft.set(format!("{current:.3}"));
        self.editing.set(Some(row));
        self.focus.update(|n| *n += 1);
    }

    pub fn cancel(&self) {
        self.editing.set(None);
    }

    pub fn commit(&self, text: &str) {
        let (Some(row), Some(value)) = (self.editing.peek(), parse_price(text)) else {
            return;
        };
        let Some(before) = self.price(row) else {
            return;
        };
        self.set(row, value);
        self.undo.update(|stack| {
            stack.push((row, before));
            if stack.len() > UNDO_DEPTH {
                stack.remove(0);
            }
        });
        self.editing.set(None);
    }

    pub fn undo(&self) {
        let mut entry = None;
        self.undo.update(|stack| entry = stack.pop());
        if let Some((row, before)) = entry {
            self.set(row, before);
        }
    }

    #[must_use]
    pub fn invalid(&self) -> Signal<bool> {
        let draft = self.draft;
        derived_eq(move || parse_price(&draft.get()).is_none())
    }

    fn set(&self, row: u32, value: f64) {
        let Ok(index) = usize::try_from(row) else {
            return;
        };
        self.prices.update(|prices| {
            if let Some(slot) = Rc::make_mut(prices).get_mut(index) {
                *slot = value;
            }
        });
    }
}

fn columns(live: Live) -> Vec<TableColumn<Row>> {
    FIELDS.iter().map(|field| column(live, field)).collect()
}

fn column(live: Live, field: &'static Field) -> TableColumn<Row> {
    let kind = field.kind;
    let mut column = TableColumn::new(field.key, header_label(field.key))
        .width(ColWidth::Fixed(field.width))
        .cell(move |row: &Row| cell(live, row, field));
    if !matches!(
        kind,
        FieldKind::Symbol | FieldKind::Name | FieldKind::DividendDate
    ) {
        column = column.numeric();
    }
    match kind {
        FieldKind::LastDone => column.sortable(move |a: &Row, b: &Row| by_price(live, a, b)),
        FieldKind::Symbol | FieldKind::Name => column
            .sortable(move |a: &Row, b: &Row| compare(a, b, kind))
            .filterable(|row: &Row, needle: &str| row.matches(needle)),
        _ => column.sortable(move |a: &Row, b: &Row| compare(a, b, kind)),
    }
}

fn by_price(live: Live, a: &Row, b: &Row) -> Ordering {
    let at = |row: &Row| u32::try_from(row.index).ok().and_then(|i| live.price(i));
    match (at(a), at(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
