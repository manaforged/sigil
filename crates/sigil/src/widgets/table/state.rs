use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use crate::reactive::{Signal, effect, signal};

use super::columns::{ColWidth, MIN_COL_WIDTH, TableColumn};
use super::derive::{compute_visible, retain_visible, reuse_identity};
use super::identity::{self, Identifiable};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDir {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortKey {
    pub column: &'static str,
    pub dir: SortDir,
}

#[derive(Clone, Debug, Default)]
pub struct ColumnLayout {
    pub order: Vec<usize>,
    pub hidden: HashSet<&'static str>,
    pub widths: HashMap<&'static str, f32>,
}

impl ColumnLayout {
    fn natural(columns: usize) -> Self {
        Self {
            order: (0..columns).collect(),
            ..Self::default()
        }
    }
}

pub(super) type RowEq<Row> = Rc<dyn Fn(&Row, &Row) -> bool>;

type Remap<Row> =
    Rc<dyn Fn(&[Row], &[Row], &HashSet<u32>, Option<u32>) -> (HashSet<u32>, Option<u32>)>;

pub struct TableState<Row: 'static> {
    rows: Signal<Arc<Vec<Row>>>,
    columns: Rc<Vec<TableColumn<Row>>>,
    sort: Signal<Option<SortKey>>,
    quick_filter: Signal<String>,
    selection: Signal<Rc<HashSet<u32>>>,
    select_anchor: Signal<Option<u32>>,
    visible: Signal<Rc<Vec<u32>>>,
    layout: Signal<Rc<ColumnLayout>>,
    remap: Option<Remap<Row>>,
    row_eq: Option<RowEq<Row>>,
}

impl<Row: 'static> Clone for TableState<Row> {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows,
            columns: Rc::clone(&self.columns),
            sort: self.sort,
            quick_filter: self.quick_filter,
            selection: self.selection,
            select_anchor: self.select_anchor,
            visible: self.visible,
            layout: self.layout,
            remap: self.remap.clone(),
            row_eq: self.row_eq.clone(),
        }
    }
}

impl<Row: 'static> TableState<Row> {
    #[must_use]
    pub fn new(rows: Vec<Row>, columns: Vec<TableColumn<Row>>) -> Self {
        let rows = signal(Arc::new(rows));
        let columns = Rc::new(columns);
        let sort = signal(None::<SortKey>);
        let quick_filter = signal(String::new());
        let derive_columns = Rc::clone(&columns);
        let mut identity = None;
        let mut previous = None;
        let visible = signal(Rc::new(Vec::new()));
        let selection = signal(Rc::new(HashSet::new()));
        let select_anchor = signal(None::<u32>);
        effect(move || {
            let key = sort.get();
            let query = quick_filter.get();
            let live_rows = rows.get();
            let indices = if key.is_none() && query.is_empty() {
                reuse_identity(live_rows.len(), &mut identity)
            } else {
                compute_visible(&live_rows, &derive_columns, key, &query, previous.as_ref())
            };
            previous = (key.is_some() && query.is_empty()).then(|| Rc::clone(&indices));
            retain_visible(&indices, selection, select_anchor);
            visible.set(indices);
        });
        let layout = signal(Rc::new(ColumnLayout::natural(columns.len())));
        Self {
            rows,
            columns,
            sort,
            quick_filter,
            selection,
            select_anchor,
            visible,
            layout,
            remap: None,
            row_eq: None,
        }
    }

    #[must_use]
    pub fn row_eq(mut self, eq: impl Fn(&Row, &Row) -> bool + 'static) -> Self {
        self.row_eq = Some(Rc::new(eq));
        self
    }

    pub(super) fn row_eq_fn(&self) -> Option<RowEq<Row>> {
        self.row_eq.clone()
    }

    #[must_use]
    pub const fn rows(&self) -> Signal<Arc<Vec<Row>>> {
        self.rows
    }

    pub fn replace_rows(&self, rows: impl Into<Arc<Vec<Row>>>) {
        let rows = rows.into();
        let previous = self.rows.peek();
        if Arc::ptr_eq(&previous, &rows) {
            return;
        }
        let (selection, anchor) = (self.selection.peek(), self.select_anchor.peek());
        #[allow(
            clippy::cast_possible_truncation,
            reason = "values are bounded to the target range before the cast"
        )]
        let (selected, anchor) = self.remap.as_ref().map_or_else(
            || identity::retain_in_range(rows.len() as u32, &selection, anchor),
            |remap| remap(&previous, &rows, &selection, anchor),
        );
        self.rows.set_always(rows);
        if *self.selection.peek() != selected {
            self.selection.set(Rc::new(selected));
        }
        self.select_anchor.set(anchor);
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<u32> {
        self.selection.get().iter().copied().min()
    }

    #[must_use]
    pub fn selected_row(&self) -> Option<Row>
    where
        Row: Clone,
    {
        let index = self.selected_index()? as usize;
        self.rows.get().get(index).cloned()
    }

    #[must_use]
    pub fn columns(&self) -> &[TableColumn<Row>] {
        &self.columns
    }

    #[must_use]
    pub const fn visible(&self) -> Signal<Rc<Vec<u32>>> {
        self.visible
    }

    #[must_use]
    pub const fn sort_key(&self) -> Signal<Option<SortKey>> {
        self.sort
    }

    pub fn toggle_sort(&self, column: &'static str) {
        if !self.is_sortable(column) {
            return;
        }
        let dir = match self.sort.peek() {
            Some(key) if key.column == column => match key.dir {
                SortDir::Ascending => Some(SortDir::Descending),
                SortDir::Descending => None,
            },
            _ => Some(SortDir::Ascending),
        };
        self.sort.set(dir.map(|dir| SortKey { column, dir }));
    }

    #[must_use]
    pub const fn quick_filter(&self) -> Signal<String> {
        self.quick_filter
    }

    #[must_use]
    pub const fn selection(&self) -> Signal<Rc<HashSet<u32>>> {
        self.selection
    }

    pub fn select_only(&self, row: u32) {
        self.selection.set(Rc::new(HashSet::from([row])));
        self.select_anchor.set(Some(row));
    }

    pub fn toggle_selected(&self, row: u32) {
        let mut set = (*self.selection.peek()).clone();
        if !set.remove(&row) {
            set.insert(row);
        }
        self.selection.set(Rc::new(set));
        self.select_anchor.set(Some(row));
    }

    pub fn select_range_to(&self, row: u32) {
        let Some(anchor) = self.select_anchor.peek() else {
            self.select_only(row);
            return;
        };
        let visible = self.visible.peek();
        let position = |needle: u32| visible.iter().position(|&index| index == needle);
        let (Some(from), Some(to)) = (position(anchor), position(row)) else {
            self.select_only(row);
            return;
        };
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        let span = visible
            .get(lo..=hi)
            .unwrap_or_default()
            .iter()
            .copied()
            .collect::<HashSet<u32>>();
        self.selection.set(Rc::new(span));
    }

    pub(super) fn anchor(&self) -> Option<u32> {
        self.select_anchor.peek()
    }

    pub fn clear_selection(&self) {
        self.selection.set(Rc::new(HashSet::new()));
        self.select_anchor.set(None);
    }

    #[must_use]
    pub const fn column_layout(&self) -> Signal<Rc<ColumnLayout>> {
        self.layout
    }

    #[must_use]
    pub fn visible_columns(&self) -> Vec<(usize, &TableColumn<Row>)> {
        let layout = self.layout.peek();
        layout
            .order
            .iter()
            .filter_map(|&index| {
                let column = self.columns.get(index)?;
                (!layout.hidden.contains(column.id())).then_some((index, column))
            })
            .collect()
    }

    pub fn move_column(&self, from: usize, to: usize) {
        self.edit_layout(|layout| {
            let len = layout.order.len();
            if from >= len || to >= len || from == to {
                return false;
            }
            let index = layout.order.remove(from);
            layout.order.insert(to, index);
            true
        });
    }

    pub fn hide_column(&self, id: &'static str) {
        if self.visible_columns().len() > 1 {
            self.edit_layout(|layout| {
                layout.hidden.insert(id);
                true
            });
        }
    }

    pub fn show_column(&self, id: &'static str) {
        self.edit_layout(|layout| layout.hidden.remove(id));
    }

    pub fn show_all_columns(&self) {
        self.edit_layout(|layout| {
            let changed = !layout.hidden.is_empty();
            layout.hidden.clear();
            changed
        });
    }

    pub fn pin_column_width(&self, id: &'static str, px: f32) {
        self.edit_layout(|layout| {
            layout.widths.insert(id, px.max(MIN_COL_WIDTH));
            true
        });
    }

    pub fn clear_column_width(&self, id: &'static str) {
        self.edit_layout(|layout| layout.widths.remove(id).is_some());
    }

    pub fn reset_columns(&self) {
        self.layout
            .set_always(Rc::new(ColumnLayout::natural(self.columns.len())));
    }

    #[must_use]
    pub fn column_width(&self, id: &'static str) -> Option<f32> {
        let layout = self.layout.peek();
        if let Some(&px) = layout.widths.get(id) {
            return Some(px);
        }
        self.columns
            .iter()
            .find(|col| col.id() == id)
            .and_then(|col| match col.col_width() {
                ColWidth::Fixed(px) => Some(px),
                ColWidth::Fraction(_) => None,
            })
    }

    pub fn sort(&self, column: &'static str, dir: SortDir) {
        if self.is_sortable(column) {
            self.sort.set(Some(SortKey { column, dir }));
        }
    }

    pub(super) fn is_sortable(&self, column: &str) -> bool {
        self.columns
            .iter()
            .any(|col| col.id() == column && col.is_sortable())
    }

    fn edit_layout(&self, edit: impl FnOnce(&mut ColumnLayout) -> bool) {
        let mut layout = (*self.layout.peek()).clone();
        if edit(&mut layout) {
            self.layout.set_always(Rc::new(layout));
        }
    }
}

impl<Row: Identifiable + 'static> TableState<Row> {
    #[must_use]
    pub fn identified(rows: Vec<Row>, columns: Vec<TableColumn<Row>>) -> Self {
        let mut state = Self::new(rows, columns);
        state.remap = Some(Rc::new(|previous, next, selected, anchor| {
            identity::remap(|row: &Row| row.id(), previous, next, selected, anchor)
        }));
        state
    }
}
