use crate::layout::Space;
use std::cmp::Ordering;
use std::rc::Rc;

use super::edit::ColumnEdit;
use crate::kernel::IntoNode;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Length, Overflow};

kittest_pub! {
    /// Smallest width a table column can be resized to.
    #[doc(hidden)]
    pub const MIN_COL_WIDTH: f32 = 48.0;
}

type CompareFn<Row> = Rc<dyn Fn(&Row, &Row) -> Ordering>;

type FilterFn<Row> = Rc<dyn Fn(&Row, &str) -> bool>;

type CellFn<Row> = Rc<dyn Fn(&Row) -> Box<dyn IntoNode>>;

/// Column width.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ColumnWidth {
    /// A width in px.
    Fixed(f32),
    /// A share of the free width, as a flex grow factor.
    Fraction(f32),
}

/// Column alignment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ColumnAlign {
    /// Content at the start edge of the column.
    #[default]
    Start,
    /// Content at the end edge of the column.
    End,
}

/// One table column.
pub struct TableColumn<Row> {
    id: &'static str,
    title: String,
    compare: Option<CompareFn<Row>>,
    filter: Option<FilterFn<Row>>,
    cell: Option<CellFn<Row>>,
    edit: Option<ColumnEdit<Row>>,
    width: ColumnWidth,
    min_width: f32,
    align: ColumnAlign,
    numeric: bool,
}

impl<Row> core::fmt::Debug for TableColumn<Row> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TableColumn")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("width", &self.width)
            .field("min_width", &self.min_width)
            .field("align", &self.align)
            .field("numeric", &self.numeric)
            .field("editable", &self.is_editable())
            .finish_non_exhaustive()
    }
}

impl<Row> Clone for TableColumn<Row> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            title: self.title.clone(),
            compare: self.compare.clone(),
            filter: self.filter.clone(),
            cell: self.cell.clone(),
            edit: self.edit.clone(),
            width: self.width,
            min_width: self.min_width,
            align: self.align,
            numeric: self.numeric,
        }
    }
}

impl<Row> TableColumn<Row> {
    /// Creates a column with the key `id` and the header text `title`.
    #[must_use]
    pub fn new(id: &'static str, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            compare: None,
            filter: None,
            cell: None,
            edit: None,
            width: ColumnWidth::Fraction(1.0),
            min_width: MIN_COL_WIDTH,
            align: ColumnAlign::Start,
            numeric: false,
        }
    }

    /// Aligns cell content to the start or the end of the column.
    #[must_use]
    pub const fn align(mut self, align: ColumnAlign) -> Self {
        self.align = align;
        self
    }

    /// Aligns the column for numbers.
    #[must_use]
    pub const fn numeric(mut self) -> Self {
        self.align = ColumnAlign::End;
        self.numeric = true;
        self
    }

    /// Lets the user sort by this column in the order that `compare` gives.
    #[must_use]
    pub fn sortable(mut self, compare: impl Fn(&Row, &Row) -> Ordering + 'static) -> Self {
        self.compare = Some(Rc::new(compare));
        self
    }

    /// Lets the user sort by this column in the order of the value that `key` returns.
    #[must_use]
    pub fn sort_by<K>(self, key: impl Fn(&Row) -> K + 'static) -> Self
    where
        K: Ord,
    {
        self.sortable(move |left, right| key(left).cmp(&key(right)))
    }

    /// Keeps a row in the quick filter when `matches` returns true for the row and the query.
    #[must_use]
    pub fn filterable(mut self, matches: impl Fn(&Row, &str) -> bool + 'static) -> Self {
        self.filter = Some(Rc::new(matches));
        self
    }

    /// Renders each cell with the node that `render` returns for the row.
    #[must_use]
    pub fn cell<N>(mut self, render: impl Fn(&Row) -> N + 'static) -> Self
    where
        N: IntoNode + 'static,
    {
        self.cell = Some(Rc::new(move |row| Box::new(render(row))));
        self
    }

    /// Lets the user edit each cell in place: `text` gives the starting text and `commit` writes the result into the row.
    ///
    /// ```
    /// use sigil::prelude::*;
    ///
    /// #[derive(Clone)]
    /// struct Person {
    ///     name: String,
    /// }
    ///
    /// fn people() -> impl IntoNode {
    ///     let name = TableColumn::new("name", "Name")
    ///         .cell(|p: &Person| text(p.name.clone()))
    ///         .editable(|p| p.name.clone(), |p, text| p.name = text);
    ///     table(TableState::new(vec![Person { name: "Ada".into() }], vec![name]))
    /// }
    /// ```
    #[must_use]
    pub fn editable(
        mut self,
        text: impl Fn(&Row) -> String + 'static,
        commit: impl Fn(&mut Row, String) + 'static,
    ) -> Self
    where
        Row: Clone + 'static,
    {
        self.edit = Some(ColumnEdit::new(text, commit));
        self
    }

    /// Returns whether the user can edit this column's cells.
    #[must_use]
    pub const fn is_editable(&self) -> bool {
        self.edit.is_some()
    }

    pub(super) const fn cell_edit(&self) -> Option<&ColumnEdit<Row>> {
        self.edit.as_ref()
    }

    /// Sets the column width; the default is `ColumnWidth::Fraction(1.0)`.
    #[must_use]
    pub const fn width(mut self, width: ColumnWidth) -> Self {
        self.width = width;
        self
    }

    /// Sets the smallest width in px of a fraction or resized column; the default is 48.
    #[must_use]
    pub const fn min_width(mut self, px: f32) -> Self {
        self.min_width = if px > 0.0 { px } else { 0.0 };
        self
    }

    /// The key that names the column in sort, width, and visibility calls.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// The header text.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub(crate) fn compare(&self) -> Option<&CompareFn<Row>> {
        self.compare.as_ref()
    }

    #[must_use]
    pub(crate) fn filter(&self) -> Option<&FilterFn<Row>> {
        self.filter.as_ref()
    }

    #[must_use]
    pub(crate) fn cell_render(&self) -> Option<&CellFn<Row>> {
        self.cell.as_ref()
    }

    #[must_use]
    pub(crate) const fn col_width(&self) -> ColumnWidth {
        self.width
    }

    #[must_use]
    pub(crate) const fn min_px(&self) -> f32 {
        self.min_width
    }

    /// Returns whether this table column is sortable.
    #[must_use]
    pub fn is_sortable(&self) -> bool {
        self.compare.is_some()
    }

    /// The edge of the column that cell content aligns to.
    #[must_use]
    pub const fn alignment(&self) -> ColumnAlign {
        self.align
    }

    /// Returns whether this table column is numeric.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        self.numeric
    }

    pub(super) fn cell_layout(
        &self,
        override_px: Option<f32>,
        height: f32,
        body_pad_x: Option<f32>,
    ) -> LayoutStyle {
        let mut style = LayoutStyle::flex_row().align_items(AlignItems::Center);
        if let Some(pad_x) = body_pad_x {
            if self.align == ColumnAlign::End {
                style = style.justify_content(JustifyContent::End);
            }
            style = style
                .padding_x(Space::px(pad_x))
                .overflow_x(Overflow::Hidden);
        }
        let floor = self.min_width.max(0.0);
        if let Some(px) = override_px {
            return style.fixed_size(px.max(floor), height).shrink(0.0);
        }
        match self.width {
            ColumnWidth::Fixed(px) => style.fixed_size(px.max(0.0), height).shrink(0.0),
            ColumnWidth::Fraction(fraction) => style
                .height(height)
                .flex_basis(Length::Px(0.0))
                .grow(fraction.max(0.0))
                .min_size(floor, height)
                .shrink(1.0),
        }
    }
}
