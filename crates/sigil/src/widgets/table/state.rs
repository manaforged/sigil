//! Headless table state engine (DR-057 D1).
//!
//! Pure logic over signals: rows in, a derived `visible` index list
//! out, with sort / quick-filter / selection each independently
//! reactive. No node tree, no renderer types — the whole engine is
//! unit-testable headless and budgeted at 100k rows.

use std::collections::HashSet;
use std::rc::Rc;

use sigil_reactive::{Signal, derived, signal};

use super::columns::TableColumn;
use super::derive::compute_visible;

/// Sort direction for one column.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDir {
    /// Comparator order.
    Ascending,
    /// Reversed comparator order.
    Descending,
}

/// The active sort: which column, which direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortKey {
    /// The [`TableColumn::id`](super::TableColumn::id) being sorted.
    pub column: &'static str,
    /// Direction.
    pub dir: SortDir,
}

/// Headless state engine for a data table.
///
/// Cheap to clone (signal handles are `Copy`; columns are shared).
/// The engine owns nothing visual: hand [`Self::visible`] to a
/// `virtual_list` (or anything else) and render rows however the
/// consumer wants.
pub struct TableState<Row: 'static> {
    rows: Signal<Rc<Vec<Row>>>,
    columns: Rc<Vec<TableColumn<Row>>>,
    sort: Signal<Option<SortKey>>,
    quick_filter: Signal<String>,
    selection: Signal<Rc<HashSet<u32>>>,
    select_anchor: Signal<Option<u32>>,
    visible: Signal<Rc<Vec<u32>>>,
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
        }
    }
}

impl<Row: 'static> TableState<Row> {
    /// Build the engine from initial rows and column definitions.
    #[must_use]
    pub fn new(rows: Vec<Row>, columns: Vec<TableColumn<Row>>) -> Self {
        let rows = signal(Rc::new(rows));
        let columns = Rc::new(columns);
        let sort = signal(None::<SortKey>);
        let quick_filter = signal(String::new());
        let derive_columns = Rc::clone(&columns);
        let visible = derived(move || {
            Rc::new(compute_visible(
                &rows.get(),
                &derive_columns,
                sort.get(),
                &quick_filter.get(),
            ))
        });
        Self {
            rows,
            columns,
            sort,
            quick_filter,
            selection: signal(Rc::new(HashSet::new())),
            select_anchor: signal(None),
            visible,
        }
    }

    /// The backing rows.
    #[must_use]
    pub const fn rows(&self) -> Signal<Rc<Vec<Row>>> {
        self.rows
    }

    /// Replace the backing rows. Sort, filter, and the derived
    /// visible list react; selection is cleared because row indices
    /// no longer name the same data.
    pub fn replace_rows(&self, rows: Vec<Row>) {
        self.selection.set(Rc::new(HashSet::new()));
        self.select_anchor.set(None);
        self.rows.set(Rc::new(rows));
    }

    /// Column definitions, in declaration order.
    #[must_use]
    pub fn columns(&self) -> &[TableColumn<Row>] {
        &self.columns
    }

    /// Visible row indices into [`Self::rows`], after quick-filter
    /// then sort. This is the signal a row virtualizer binds to.
    #[must_use]
    pub const fn visible(&self) -> Signal<Rc<Vec<u32>>> {
        self.visible
    }

    /// The active sort key signal.
    #[must_use]
    pub const fn sort(&self) -> Signal<Option<SortKey>> {
        self.sort
    }

    /// Cycle a column's sort: none → ascending → descending → none.
    /// Ignored for columns without a comparator.
    pub fn toggle_sort(&self, column: &'static str) {
        let sortable = self
            .columns
            .iter()
            .any(|col| col.id() == column && col.is_sortable());
        if !sortable {
            return;
        }
        let next = match self.sort.peek() {
            Some(key) if key.column == column => match key.dir {
                SortDir::Ascending => Some(SortKey {
                    column,
                    dir: SortDir::Descending,
                }),
                SortDir::Descending => None,
            },
            _ => Some(SortKey {
                column,
                dir: SortDir::Ascending,
            }),
        };
        self.sort.set(next);
    }

    /// The quick-filter text signal. Write through it directly:
    /// `state.quick_filter().set(text)`. Empty text shows every row.
    #[must_use]
    pub const fn quick_filter(&self) -> Signal<String> {
        self.quick_filter
    }

    /// Selected row indices (into [`Self::rows`], not the visible list).
    #[must_use]
    pub const fn selection(&self) -> Signal<Rc<HashSet<u32>>> {
        self.selection
    }

    /// Select exactly `row`, clearing any previous selection, and
    /// anchor future range-selects here.
    pub fn select_only(&self, row: u32) {
        self.selection.set(Rc::new(HashSet::from([row])));
        self.select_anchor.set(Some(row));
    }

    /// Toggle `row` in the selection (ctrl/cmd-click shape). The
    /// anchor moves to `row`.
    pub fn toggle_selected(&self, row: u32) {
        let mut set = (*self.selection.peek()).clone();
        if !set.remove(&row) {
            set.insert(row);
        }
        self.selection.set(Rc::new(set));
        self.select_anchor.set(Some(row));
    }

    /// Select the visible span between the anchor and `row`
    /// (shift-click shape). Falls back to [`Self::select_only`] when
    /// no anchor exists. The anchor stays put so the span can pivot.
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

    /// Clear the selection and anchor.
    pub fn clear_selection(&self) {
        self.selection.set(Rc::new(HashSet::new()));
        self.select_anchor.set(None);
    }
}
