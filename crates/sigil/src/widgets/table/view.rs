//! The `table(state)` view — sticky header + virtualised body (DR-057 D2).
//!
//! Assembles the pieces the headless core deliberately never knew
//! about: a pinned header row ([`Role::ColumnHeader`] cells with sort
//! indicators) above a `virtual_list` body bound to
//! [`TableState::visible`], so filtering, sorting, and scrolling all
//! advance in place at viewport cost, never whole-table rebuilds.
//!
//! ```rust,no_run
//! use sigil::prelude::*;
//! use sigil::widgets::{TableColumn, TableState};
//!
//! #[derive(Clone)]
//! struct Row {
//!     name: String,
//! }
//!
//! let state = TableState::new(
//!     vec![Row { name: "alpha".into() }],
//!     vec![
//!         TableColumn::new("name", "Name")
//!             .sortable(|a: &Row, b: &Row| a.name.cmp(&b.name))
//!             .width(ColWidth::Fraction(1.0))
//!             .cell(|row: &Row| container().child(label(row.name.clone()))),
//!     ],
//! );
//! let view = table(state).row_height(24.0).viewport_height(400.0);
//! ```

use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::LayoutStyle;
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::derived;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::virtual_list::virtual_list;
use super::columns::{ColWidth, TableColumn};
use super::header::header_row;
use super::row::body_row;
use super::state::TableState;

/// Default body row height in logical pixels.
const DEFAULT_ROW_HEIGHT: f32 = 24.0;
/// Default header row height in logical pixels.
const DEFAULT_HEADER_HEIGHT: f32 = 28.0;
/// Default body viewport height in logical pixels.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 480.0;
/// Default table width used to resolve `Fraction` columns when no
/// explicit [`viewport_width`](TableBuilder::viewport_width) is set.
const DEFAULT_WIDTH: f32 = 640.0;

/// Builder returned by [`table`].
pub struct TableBuilder<Row: 'static> {
    state: TableState<Row>,
    row_height: f32,
    header_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    layout_overrides: LayoutOverrides,
}

/// Build a data-table view over a [`TableState`].
///
/// The root is a [`Role::Grid`] with reactive row count; the sticky
/// header sits above a windowed body that materializes only the
/// viewport's rows. Sorting (header click / Enter / Space), selection
/// (row click with the usual modifier shapes), quick-filtering, and
/// wheel scrolling all flow through the shared `TableState`.
#[must_use]
pub const fn table<Row: 'static>(state: TableState<Row>) -> TableBuilder<Row> {
    TableBuilder {
        state,
        row_height: DEFAULT_ROW_HEIGHT,
        header_height: DEFAULT_HEADER_HEIGHT,
        viewport_height: DEFAULT_VIEWPORT_HEIGHT,
        viewport_width: None,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl<Row: 'static> TableBuilder<Row> {
    /// Set the body row height in logical pixels.
    #[must_use]
    pub const fn row_height(mut self, px: f32) -> Self {
        self.row_height = px;
        self
    }

    /// Set the header row height in logical pixels.
    #[must_use]
    pub const fn header_height(mut self, px: f32) -> Self {
        self.header_height = px;
        self
    }

    /// Set the body viewport height in logical pixels.
    #[must_use]
    pub const fn viewport_height(mut self, px: f32) -> Self {
        self.viewport_height = px;
        self
    }

    /// Set the table width in logical pixels. `Fraction` columns
    /// resolve against this width (after `Fixed` columns take their
    /// pixels); the header and the body viewport share it. Defaults to
    /// 640 px.
    #[must_use]
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let state = self.state;
        let width = self.viewport_width.unwrap_or(DEFAULT_WIDTH);
        let widths = Rc::new(resolve_widths(state.columns(), width));
        let body = virtual_list(
            Vec::new(),
            body_row(state.clone(), Rc::clone(&widths), self.row_height),
        )
        .items_bind(state.visible())
        .row_height(self.row_height)
        .viewport_height(self.viewport_height)
        .viewport_width(width);
        let root_state = state.clone();
        container()
            .role(Role::Grid)
            .row_count(derived(move || root_state.visible().get().len() + 1))
            .column_count(state.columns().len())
            .layout(
                self.layout_overrides
                    .apply(LayoutStyle::flex_column().width(width)),
            )
            .child(header_row(&state, &widths, self.header_height))
            .child(body)
    }
}

impl<Row: 'static> IntoNode for TableBuilder<Row> {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

impl<Row: 'static> WidgetLayoutExt for TableBuilder<Row> {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

/// Resolve each column to a concrete pixel width: `Fixed` columns
/// take their pixels, `Fraction` columns split what remains of
/// `total` in proportion to their fraction.
fn resolve_widths<Row>(columns: &[TableColumn<Row>], total: f32) -> Vec<f32> {
    let mut fixed = 0.0;
    let mut fractions = 0.0;
    for column in columns {
        match column.col_width() {
            ColWidth::Fixed(px) => fixed += px.max(0.0),
            ColWidth::Fraction(fraction) => fractions += fraction.max(0.0),
        }
    }
    let remaining = (total - fixed).max(0.0);
    columns
        .iter()
        .map(|column| match column.col_width() {
            ColWidth::Fixed(px) => px.max(0.0),
            ColWidth::Fraction(fraction) if fractions > 0.0 => {
                remaining * fraction.max(0.0) / fractions
            }
            ColWidth::Fraction(_) => 0.0,
        })
        .collect()
}
