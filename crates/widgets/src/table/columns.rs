//! Column definitions for the headless table core (DR-057 D1).

use std::cmp::Ordering;
use std::rc::Rc;

use sigil_kernel::IntoNode;
use sigil_layout::prelude::{AlignItems, LayoutStyle, Overflow};

/// Floor for a user-resized column, in logical pixels.
pub const MIN_COL_WIDTH: f32 = 48.0;

/// Shared ascending comparator behind `Rc` so columns stay cheap to clone.
type CompareFn<Row> = Rc<dyn Fn(&Row, &Row) -> Ordering>;

/// Shared quick-filter predicate behind `Rc` so columns stay cheap to clone.
type FilterFn<Row> = Rc<dyn Fn(&Row, &str) -> bool>;

/// Shared cell renderer behind `Rc` so columns stay cheap to clone.
type CellFn<Row> = Rc<dyn Fn(&Row) -> Box<dyn IntoNode>>;

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
    min_width: f32,
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
            min_width: self.min_width,
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
            min_width: MIN_COL_WIDTH,
        }
    }

    /// Make the column sortable with `compare` (ascending order; the
    /// state engine reverses for descending).
    #[must_use]
    pub fn sortable(mut self, compare: impl Fn(&Row, &Row) -> Ordering + 'static) -> Self {
        self.compare = Some(Rc::new(compare));
        self
    }

    /// Sort by a key extracted from each row. Prefer this over
    /// [`Self::sortable`] when the order is `Ord` on one field.
    #[must_use]
    pub fn sort_by<K>(self, key: impl Fn(&Row) -> K + 'static) -> Self
    where
        K: Ord,
    {
        self.sortable(move |left, right| key(left).cmp(&key(right)))
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
    /// Accepts any [`IntoNode`] — `text`, `chip`, `label`, a
    /// `container`. The `table()` view wraps it in a `GridCell`
    /// sized to the column. Columns without a renderer paint empty.
    #[must_use]
    pub fn cell<N>(mut self, render: impl Fn(&Row) -> N + 'static) -> Self
    where
        N: IntoNode + 'static,
    {
        self.cell = Some(Rc::new(move |row| Box::new(render(row))));
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

    /// Floor for this column in logical pixels. `Fraction` columns
    /// shrink no smaller than this. Defaults to [`MIN_COL_WIDTH`].
    #[must_use]
    pub const fn min_width(mut self, px: f32) -> Self {
        self.min_width = if px > 0.0 { px } else { 0.0 };
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

    /// Pixel floor used by the `table()` view when resolving flex cells.
    #[must_use]
    pub const fn min_width_px(&self) -> f32 {
        self.min_width
    }

    /// Pixel floor used by the `table()` view when resolving flex cells.
    #[must_use]
    pub(crate) const fn min_px(&self) -> f32 {
        self.min_width
    }

    /// Whether [`Self::sortable`] was configured.
    #[must_use]
    pub fn is_sortable(&self) -> bool {
        self.compare.is_some()
    }
}

/// Flex layout for one header or body cell.
pub(super) fn flex_cell(
    policy: ColWidth,
    override_px: Option<f32>,
    height: f32,
    clip: bool,
    min_px: f32,
) -> LayoutStyle {
    let mut style = LayoutStyle::flex_row().align_items(AlignItems::Center);
    if clip {
        style = style.padding_x(8.0);
    }
    if clip {
        style = style.overflow_x(Overflow::Hidden);
    }
    let floor = min_px.max(0.0);
    if let Some(px) = override_px {
        return style.fixed_size(px.max(floor), height).shrink(0.0);
    }
    match policy {
        ColWidth::Fixed(px) => style.fixed_size(px.max(0.0), height).shrink(0.0),
        ColWidth::Fraction(fraction) => style
            .height(height)
            .grow(fraction.max(0.0))
            .min_size(floor, height)
            .shrink(1.0),
    }
}
