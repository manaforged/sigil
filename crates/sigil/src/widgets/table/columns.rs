//! Column definitions for the headless table core (DR-057 D1).

use std::cmp::Ordering;
use std::rc::Rc;

use sigil_kernel::NodeBuilder;

/// Shared ascending comparator behind `Rc` so columns stay cheap to clone.
type CompareFn<Row> = Rc<dyn Fn(&Row, &Row) -> Ordering>;

/// Shared quick-filter predicate behind `Rc` so columns stay cheap to clone.
type FilterFn<Row> = Rc<dyn Fn(&Row, &str) -> bool>;

/// Shared cell renderer behind `Rc` so columns stay cheap to clone.
type CellFn<Row> = Rc<dyn Fn(&Row) -> NodeBuilder>;

/// Width policy for one column.
///
/// The `table()` view resolves every column to a concrete pixel width
/// at build time: [`Fixed`](ColWidth::Fixed) columns take their pixels
/// first, then [`Fraction`](ColWidth::Fraction) columns split what
/// remains of the table width in proportion to their fraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColWidth {
    /// Fixed logical-pixel width.
    Fixed(f32),
    /// Share of the remaining table width after all `Fixed` columns
    /// take their pixels. `Fraction(2.0)` gets twice the pixels of
    /// `Fraction(1.0)`.
    Fraction(f32),
}

/// One column of a [`TableState`](super::TableState): identity, header
/// title, and the behaviours the state engine and the `table()` view
/// consult.
///
/// The state engine reads *logic only* — the comparator and the filter
/// predicate — keeping it unit-testable without a node tree (the
/// TanStack-shape separation locked in DR-057). The presentation hints
/// ([`cell`](Self::cell) renderer and [`width`](Self::width) policy)
/// travel on the same column so header and body cells stay in sync,
/// but the engine never consults them.
pub struct TableColumn<Row> {
    id: &'static str,
    title: String,
    compare: Option<CompareFn<Row>>,
    filter: Option<FilterFn<Row>>,
    cell: Option<CellFn<Row>>,
    width: ColWidth,
}

impl<Row> Clone for TableColumn<Row> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            title: self.title.clone(),
            compare: self.compare.clone(),
            filter: self.filter.clone(),
            cell: self.cell.clone(),
            width: self.width,
        }
    }
}

impl<Row> TableColumn<Row> {
    /// Define a column with a stable identifier and a header title.
    ///
    /// The `id` names the column in sort state and keyboard routing;
    /// it must be unique within one table.
    #[must_use]
    pub fn new(id: &'static str, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            compare: None,
            filter: None,
            cell: None,
            width: ColWidth::Fraction(1.0),
        }
    }

    /// Make the column sortable with `compare` (ascending order; the
    /// state engine reverses for descending).
    #[must_use]
    pub fn sortable(mut self, compare: impl Fn(&Row, &Row) -> Ordering + 'static) -> Self {
        self.compare = Some(Rc::new(compare));
        self
    }

    /// Let the quick filter consult this column: `matches(row, query)`
    /// returns whether `row` survives for `query`. A row is visible
    /// when ANY filterable column matches (the spreadsheet shape).
    #[must_use]
    pub fn filterable(mut self, matches: impl Fn(&Row, &str) -> bool + 'static) -> Self {
        self.filter = Some(Rc::new(matches));
        self
    }

    /// Render one body cell of this column.
    ///
    /// The `table()` view wraps the returned node in a `GridCell`
    /// container sized to the column's resolved width. Columns without
    /// a renderer paint an empty cell.
    #[must_use]
    pub fn cell(mut self, render: impl Fn(&Row) -> NodeBuilder + 'static) -> Self {
        self.cell = Some(Rc::new(render));
        self
    }

    /// Set the column width policy. Defaults to
    /// [`ColWidth::Fraction(1.0)`](ColWidth::Fraction) — every column
    /// shares the table width equally.
    #[must_use]
    pub const fn width(mut self, width: ColWidth) -> Self {
        self.width = width;
        self
    }

    /// Stable column identifier.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Header title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The ascending comparator, when the column is sortable.
    #[must_use]
    pub(crate) fn compare(&self) -> Option<&CompareFn<Row>> {
        self.compare.as_ref()
    }

    /// The quick-filter predicate, when the column is filterable.
    #[must_use]
    pub(crate) fn filter(&self) -> Option<&FilterFn<Row>> {
        self.filter.as_ref()
    }

    /// The body-cell renderer, when [`Self::cell`] was configured.
    #[must_use]
    pub(crate) fn cell_render(&self) -> Option<&CellFn<Row>> {
        self.cell.as_ref()
    }

    /// The width policy resolved by the `table()` view.
    #[must_use]
    pub(crate) const fn col_width(&self) -> ColWidth {
        self.width
    }

    /// Whether [`Self::sortable`] was configured.
    #[must_use]
    pub fn is_sortable(&self) -> bool {
        self.compare.is_some()
    }
}
