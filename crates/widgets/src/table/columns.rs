use std::cmp::Ordering;
use std::rc::Rc;

use sigil_kernel::IntoNode;
use sigil_layout::prelude::{AlignItems, LayoutStyle, Overflow};

pub const MIN_COL_WIDTH: f32 = 48.0;

type CompareFn<Row> = Rc<dyn Fn(&Row, &Row) -> Ordering>;

type FilterFn<Row> = Rc<dyn Fn(&Row, &str) -> bool>;

type CellFn<Row> = Rc<dyn Fn(&Row) -> Box<dyn IntoNode>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColWidth {
    Fixed(f32),
    Fraction(f32),
}

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

    #[must_use]
    pub fn sortable(mut self, compare: impl Fn(&Row, &Row) -> Ordering + 'static) -> Self {
        self.compare = Some(Rc::new(compare));
        self
    }

    #[must_use]
    pub fn sort_by<K>(self, key: impl Fn(&Row) -> K + 'static) -> Self
    where
        K: Ord,
    {
        self.sortable(move |left, right| key(left).cmp(&key(right)))
    }

    #[must_use]
    pub fn filterable(mut self, matches: impl Fn(&Row, &str) -> bool + 'static) -> Self {
        self.filter = Some(Rc::new(matches));
        self
    }

    #[must_use]
    pub fn cell<N>(mut self, render: impl Fn(&Row) -> N + 'static) -> Self
    where
        N: IntoNode + 'static,
    {
        self.cell = Some(Rc::new(move |row| Box::new(render(row))));
        self
    }

    #[must_use]
    pub const fn width(mut self, width: ColWidth) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub const fn min_width(mut self, px: f32) -> Self {
        self.min_width = if px > 0.0 { px } else { 0.0 };
        self
    }

    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

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
    pub(crate) const fn col_width(&self) -> ColWidth {
        self.width
    }

    #[must_use]
    pub const fn min_width_px(&self) -> f32 {
        self.min_width
    }

    #[must_use]
    pub(crate) const fn min_px(&self) -> f32 {
        self.min_width
    }

    #[must_use]
    pub fn is_sortable(&self) -> bool {
        self.compare.is_some()
    }
}

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
