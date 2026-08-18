//! Headless table state engine (DR-057 D1).
//!
//! Pure logic over signals: rows in, a derived `visible` index list
//! out, with sort / quick-filter / selection each independently
//! reactive. No node tree, no renderer types — the whole engine is
//! unit-testable headless and budgeted at 100k rows.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use sigil_reactive::{Signal, derived, signal};

use super::columns::{ColWidth, MIN_COL_WIDTH, TableColumn};
use super::derive::compute_visible;
use super::identity::{self, Identifiable};

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

/// Visual column layout: order, hidden set, and pixel width overrides.
#[derive(Clone, Debug, Default)]
pub struct ColumnLayout {
    /// Visual order as indices into the column definition list.
    pub order: Vec<usize>,
    /// Hidden column ids.
    pub hidden: HashSet<&'static str>,
    /// Pixel width overrides keyed by column id. Missing keys follow
    /// the column's [`ColWidth`](super::columns::ColWidth) policy.
    pub widths: HashMap<&'static str, f32>,
}

type Remap<Row> =
    Rc<dyn Fn(&[Row], &[Row], &HashSet<u32>, Option<u32>) -> (HashSet<u32>, Option<u32>)>;

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
    layout: Signal<Rc<ColumnLayout>>,
    remap: Option<Remap<Row>>,
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
        }
    }
}

impl<Row: 'static> TableState<Row> {
    /// Build the engine from rows and columns. Selection is by index.
    /// If `Row: Identifiable`, use [`Self::identified`] so sort and
    /// replace keep the row you clicked.
    #[must_use]
    pub fn new(rows: Vec<Row>, columns: Vec<TableColumn<Row>>) -> Self {
        let rows = signal(Rc::new(rows));
        let columns = Rc::new(columns);
        let sort = signal(None::<SortKey>);
        let quick_filter = signal(String::new());
        let derive_columns = Rc::clone(&columns);
        let identity = Rc::new(RefCell::new(None));
        let visible = derived(move || {
            compute_visible(
                &rows.get(),
                &derive_columns,
                sort.get(),
                &quick_filter.get(),
                &identity,
            )
        });
        let order = (0..columns.len()).collect();
        Self {
            rows,
            columns,
            sort,
            quick_filter,
            selection: signal(Rc::new(HashSet::new())),
            select_anchor: signal(None),
            visible,
            layout: signal(Rc::new(ColumnLayout {
                order,
                hidden: HashSet::new(),
                widths: HashMap::new(),
            })),
            remap: None,
        }
    }

    /// The backing rows.
    #[must_use]
    pub const fn rows(&self) -> Signal<Rc<Vec<Row>>> {
        self.rows
    }

    /// Replace the backing rows. Sort, filter, and the derived
    /// visible list react. [`Self::identified`] remaps selection by
    /// [`Identifiable::id`]. Otherwise any index that still names a
    /// hole stays; out-of-range indices drop.
    pub fn replace_rows(&self, rows: Vec<Row>) {
        let previous = self.rows.peek();
        // Table row counts are far below the u32 range.
        #[allow(clippy::cast_possible_truncation)]
        let (selected, anchor) = self.remap.as_ref().map_or_else(
            || {
                identity::retain_in_range(
                    rows.len() as u32,
                    &self.selection.peek(),
                    self.select_anchor.peek(),
                )
            },
            |remap| {
                remap(
                    &previous,
                    &rows,
                    &self.selection.peek(),
                    self.select_anchor.peek(),
                )
            },
        );
        self.rows.set_always(Rc::new(rows));
        self.selection.set(Rc::new(selected));
        self.select_anchor.set(anchor);
    }

    /// First selected row index, if any. Reads `selection` and `rows`
    /// with [`Signal::get`] so a `derived` that calls this stays live.
    #[must_use]
    pub fn selected_index(&self) -> Option<u32> {
        self.selection.get().iter().copied().min()
    }

    /// Clone of the first selected row, if any.
    #[must_use]
    pub fn selected_row(&self) -> Option<Row>
    where
        Row: Clone,
    {
        let index = self.selected_index()? as usize;
        self.rows.get().get(index).cloned()
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
    pub const fn sort_key(&self) -> Signal<Option<SortKey>> {
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

    /// Visual column layout signal (order, hidden, width overrides).
    #[must_use]
    pub const fn column_layout(&self) -> Signal<Rc<ColumnLayout>> {
        self.layout
    }

    /// Visible columns in visual order: `(definition_index, column)`.
    #[must_use]
    pub fn visible_columns(&self) -> Vec<(usize, &TableColumn<Row>)> {
        let layout = self.layout.peek();
        layout
            .order
            .iter()
            .filter_map(|&index| {
                let column = self.columns.get(index)?;
                if layout.hidden.contains(column.id()) {
                    None
                } else {
                    Some((index, column))
                }
            })
            .collect()
    }

    /// Move the column at visual index `from` to visual index `to`.
    pub fn move_column(&self, from: usize, to: usize) {
        let mut layout = (*self.layout.peek()).clone();
        let len = layout.order.len();
        if from >= len || to >= len || from == to {
            return;
        }
        let index = layout.order.remove(from);
        layout.order.insert(to, index);
        self.layout.set_always(Rc::new(layout));
    }

    /// Hide `id`. The last visible column stays visible.
    pub fn hide_column(&self, id: &'static str) {
        let visible = self.visible_columns().len();
        if visible <= 1 {
            return;
        }
        let mut layout = (*self.layout.peek()).clone();
        layout.hidden.insert(id);
        self.layout.set_always(Rc::new(layout));
    }

    /// Show a previously hidden column.
    pub fn show_column(&self, id: &'static str) {
        let mut layout = (*self.layout.peek()).clone();
        if layout.hidden.remove(id) {
            self.layout.set_always(Rc::new(layout));
        }
    }

    /// Show every column.
    pub fn show_all_columns(&self) {
        let mut layout = (*self.layout.peek()).clone();
        if layout.hidden.is_empty() {
            return;
        }
        layout.hidden.clear();
        self.layout.set_always(Rc::new(layout));
    }

    /// Pin `id` to `px` logical pixels (clamped to [`MIN_COL_WIDTH`]).
    pub fn pin_column_width(&self, id: &'static str, px: f32) {
        let mut layout = (*self.layout.peek()).clone();
        layout.widths.insert(id, px.max(MIN_COL_WIDTH));
        self.layout.set_always(Rc::new(layout));
    }

    /// Drop the pixel override so `id` follows its width policy again.
    pub fn clear_column_width(&self, id: &'static str) {
        let mut layout = (*self.layout.peek()).clone();
        if layout.widths.remove(id).is_some() {
            self.layout.set_always(Rc::new(layout));
        }
    }

    /// Restore declaration order, visibility, and width policies.
    pub fn reset_columns(&self) {
        self.layout.set_always(Rc::new(ColumnLayout {
            order: (0..self.columns.len()).collect(),
            hidden: HashSet::new(),
            widths: HashMap::new(),
        }));
    }

    /// Current pixel width of `id` when it is pinned or `Fixed`.
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

    /// Apply a sort. Define the comparator with
    /// [`TableColumn::sort_by`](super::TableColumn::sort_by).
    pub fn sort(&self, column: &'static str, dir: SortDir) {
        let sortable = self
            .columns
            .iter()
            .any(|col| col.id() == column && col.is_sortable());
        if !sortable {
            return;
        }
        self.sort.set(Some(SortKey { column, dir }));
    }
}

impl<Row: Identifiable + 'static> TableState<Row> {
    /// Build a table that selects by [`Identifiable::id`].
    ///
    /// [`Self::replace_rows`] keeps the row you clicked when the
    /// vector is replaced or reordered. Use [`Self::new`] when the
    /// row has no stable id.
    #[must_use]
    pub fn identified(rows: Vec<Row>, columns: Vec<TableColumn<Row>>) -> Self {
        let mut state = Self::new(rows, columns);
        state.remap = Some(Rc::new(|previous, next, selected, anchor| {
            identity::remap(|row: &Row| row.id(), previous, next, selected, anchor)
        }));
        state
    }
}
