//! The `table(state)` view — sticky header + virtualised body (DR-057 D2).
//!
//! Assembles the pieces the headless core deliberately never knew
//! about: a pinned header row ([`Role::ColumnHeader`] cells with sort
//! indicators) above a `virtual_list` body bound to
//! [`TableState::visible`], so filtering, sorting, and scrolling all
//! advance in place at viewport cost, never whole-table rebuilds.
//!
//! Columns fill the table with flex: `Fixed` keeps its pixels,
//! `Fraction` takes the leftover. Users resize, reorder, hide, and
//! right-click through the header and row chrome.
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
//!             .sort_by(|row: &Row| row.name.clone())
//!             .width(ColWidth::Fraction(1.0))
//!             .cell(|row: &Row| label(row.name.clone())),
//!     ],
//! );
//! let view = table(state).row_height(24.0).viewport_height(400.0);
//! ```

use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::LayoutStyle;
use sigil_layout::prelude::{LayoutBuilderExt, Overflow};
use sigil_reactive::derived;
use sigil_style::NodeStyleExt;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::menu::{MenuItem, menu};
use super::super::virtual_list::virtual_list;
use super::chrome::{ContextKind, RowMenu, TableChrome};
use super::header::header_row;
use super::row::body_row;
use super::state::{SortDir, TableState};

/// Default body row height in logical pixels.
const DEFAULT_ROW_HEIGHT: f32 = 36.0;
/// Default header row height in logical pixels.
const DEFAULT_HEADER_HEIGHT: f32 = 36.0;
/// Default body viewport height in logical pixels.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 480.0;

/// Row and header height set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Density {
    /// 28 px rows, 32 px header.
    Compact,
    /// 36 px rows and header.
    #[default]
    Default,
}

impl Density {
    const fn row(self) -> f32 {
        match self {
            Self::Compact => 28.0,
            Self::Default => 36.0,
        }
    }

    const fn header(self) -> f32 {
        match self {
            Self::Compact => 32.0,
            Self::Default => 36.0,
        }
    }
}

/// Builder returned by [`table`].
pub struct TableBuilder<Row: 'static> {
    state: TableState<Row>,
    row_height: f32,
    header_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    viewport_fill: bool,
    row_menu: Option<RowMenu<Row>>,
    empty: Option<Rc<dyn Fn() -> NodeBuilder>>,
    layout_overrides: LayoutOverrides,
}

/// Build a data-table view over a [`TableState`].
///
/// The root is a [`Role::Grid`] with reactive row count; the sticky
/// header sits above a windowed body that materializes only the
/// viewport's rows. Sorting (header click / Enter / Space), selection
/// (row click with the usual modifier shapes), quick-filtering, wheel
/// scrolling, column resize / reorder / hide, and right-click menus
/// all flow through the shared `TableState`.
#[must_use]
pub const fn table<Row: 'static>(state: TableState<Row>) -> TableBuilder<Row> {
    TableBuilder {
        state,
        row_height: DEFAULT_ROW_HEIGHT,
        header_height: DEFAULT_HEADER_HEIGHT,
        viewport_height: DEFAULT_VIEWPORT_HEIGHT,
        viewport_width: None,
        viewport_fill: false,
        row_menu: None,
        empty: None,
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

    /// Set the table width in logical pixels. When omitted the table
    /// stretches to the host's layout constraint so `Fraction` columns
    /// fill leftover space.
    #[must_use]
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    /// Fill the parent pane on both axes. Overrides explicit
    /// [`viewport_width`](Self::viewport_width) /
    /// [`viewport_height`](Self::viewport_height).
    #[must_use]
    pub const fn viewport_fill(mut self) -> Self {
        self.viewport_fill = true;
        self
    }

    /// Build the row context menu from the clicked row.
    #[must_use]
    pub fn row_menu(mut self, build: impl Fn(&Row) -> Vec<MenuItem> + 'static) -> Self {
        self.row_menu = Some(Rc::new(build));
        self
    }

    /// Set row and header heights together.
    #[must_use]
    pub const fn density(mut self, density: Density) -> Self {
        self.row_height = density.row();
        self.header_height = density.header();
        self
    }

    /// Node shown when no rows survive the filter.
    #[must_use]
    pub fn empty(mut self, build: impl Fn() -> NodeBuilder + 'static) -> Self {
        self.empty = Some(Rc::new(build));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let state = self.state;
        let chrome = TableChrome::new(self.row_menu);
        let row_height = self.row_height;
        let viewport_fill = self.viewport_fill;
        let viewport_height = self.viewport_height;
        let viewport_width = self.viewport_width;
        let empty = self.empty;
        let root_state = state.clone();
        let mut root_layout = LayoutStyle::flex_column()
            .fill_width()
            .overflow_x(Overflow::Hidden);
        if viewport_fill {
            root_layout = root_layout.grow(1.0).fill_height();
        } else if let Some(width) = viewport_width {
            root_layout = root_layout.width(width);
        }
        let menu_chrome = chrome.clone();
        let menu_state = state.clone();
        let empty_state = state.clone();
        let body = virtual_list(
            state.visible(),
            body_row(state.clone(), row_height, chrome.clone()),
        )
        .key_by(|_, id| u64::from(*id))
        .row_height(row_height);
        let body = if viewport_fill {
            body.viewport_fill()
        } else {
            let body = body.viewport_height(viewport_height);
            match viewport_width {
                Some(width) => body.viewport_width(width),
                None => body,
            }
        };
        container()
            .role(Role::Grid)
            .row_count(derived(move || root_state.visible().get().len() + 1))
            .column_count(state.columns().len())
            .class("border border-gray-6 rounded-md")
            .layout(self.layout_overrides.apply(root_layout))
            .child(header_row(&state, self.header_height, &chrome))
            .child(body)
            .child(container().children_reactive(move || {
                if empty_state.visible().get().is_empty() {
                    vec![empty_slot(empty.as_ref())]
                } else {
                    Vec::new()
                }
            }))
            .child(context_overlay(menu_state, menu_chrome))
    }
}

fn empty_slot(empty: Option<&Rc<dyn Fn() -> NodeBuilder>>) -> NodeBuilder {
    empty.map_or_else(default_empty, |build| build())
}

fn default_empty() -> NodeBuilder {
    use super::super::shared::label;
    use sigil_layout::prelude::{AlignItems, JustifyContent};
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .padding(24.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .child(label("No rows").class("text-sm text-gray-11"))
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

fn context_overlay<Row: 'static>(state: TableState<Row>, chrome: TableChrome<Row>) -> NodeBuilder {
    container().children_reactive(move || {
        if !chrome.open.get() {
            return Vec::new();
        }
        let (x, y) = chrome.pos.get();
        let Some(kind) = chrome.kind.get() else {
            return Vec::new();
        };
        let items = match kind {
            ContextKind::Header(id) => header_menu(&state, id),
            ContextKind::Row(index) => row_menu_items(&state, &chrome, index),
        };
        if items.is_empty() {
            return Vec::new();
        }
        let mut panel = menu(chrome.open).label("Table menu").anchor(x, y);
        for item in items {
            panel = panel.item(item);
        }
        vec![panel.into_node_builder()]
    })
}

/// Header menu: sort, hide, autosize, move, show all.
fn header_menu<Row: 'static>(state: &TableState<Row>, id: &'static str) -> Vec<MenuItem> {
    let sortable = state
        .columns()
        .iter()
        .any(|col| col.id() == id && col.is_sortable());
    let order = state.column_layout().peek().order.clone();
    let visual = state
        .visible_columns()
        .into_iter()
        .position(|(_, col)| col.id() == id);
    let mut items = Vec::new();
    if sortable {
        let asc = state.clone();
        let desc = state.clone();
        items.push(MenuItem::new("Sort ascending").select(move || {
            asc.sort(id, SortDir::Ascending);
        }));
        items.push(MenuItem::new("Sort descending").select(move || {
            desc.sort(id, SortDir::Descending);
        }));
    }
    let hide = state.clone();
    items.push(MenuItem::new("Hide column").select(move || hide.hide_column(id)));
    let autosize = state.clone();
    items.push(MenuItem::new("Autosize").select(move || autosize.clear_column_width(id)));
    if let Some(from) = visual {
        if from > 0 {
            let left = state.clone();
            items.push(MenuItem::new("Move left").select(move || left.move_column(from, from - 1)));
        }
        if from + 1 < order.len() {
            let right = state.clone();
            items.push(
                MenuItem::new("Move right").select(move || right.move_column(from, from + 1)),
            );
        }
    }
    let show = state.clone();
    items.push(MenuItem::new("Show all columns").select(move || show.show_all_columns()));
    items
}

fn row_menu_items<Row: 'static>(
    state: &TableState<Row>,
    chrome: &TableChrome<Row>,
    index: u32,
) -> Vec<MenuItem> {
    let Some(build) = &chrome.row_menu else {
        return Vec::new();
    };
    let rows = state.rows().peek();
    let Some(row) = rows.get(index as usize) else {
        return Vec::new();
    };
    build(row)
}
