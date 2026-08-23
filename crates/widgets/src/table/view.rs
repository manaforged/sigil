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
//! right-click through the header and row chrome. When the resolved
//! columns are wider than the viewport, the table scrolls horizontally:
//! trackpad/wheel x deltas shift the header and body in lockstep, and a
//! thin scrollbar appears along the bottom.
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

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::LayoutStyle;
use sigil_layout::prelude::{LayoutBuilderExt, Overflow};
use sigil_reactive::derived;
use sigil_style::{ClassSource, NodeStyleExt};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::menu::MenuItem;
use super::super::scroll::wheel_x;
use super::super::shared::BuilderMutator;
use super::super::virtual_list::virtual_list;
use super::chrome::{RowMenu, TableChrome, context_overlay};
use super::header::header_row;
use super::hscroll;
use super::row::{RowSpec, body_row};
use super::select;
use super::state::TableState;

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
    select: bool,
    mutators: Vec<BuilderMutator>,
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
pub fn table<Row: 'static>(state: TableState<Row>) -> TableBuilder<Row> {
    TableBuilder {
        state,
        row_height: DEFAULT_ROW_HEIGHT,
        header_height: DEFAULT_HEADER_HEIGHT,
        viewport_height: DEFAULT_VIEWPORT_HEIGHT,
        viewport_width: None,
        viewport_fill: false,
        row_menu: None,
        empty: None,
        select: false,
        mutators: Vec::new(),
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

    /// Prepend a fixed-width checkbox column wired to [`TableState`]
    /// selection. A row checkbox toggles that row; the header checkbox
    /// selects every visible row (or deselects them when all are
    /// selected) and shows a dash while the selection is partial.
    #[must_use]
    pub const fn selectable_checkboxes(mut self) -> Self {
        self.select = true;
        self
    }

    /// Add utility classes after the table root defaults (`border`,
    /// `rounded-md`, `bg-surface`).
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let state = self.state;
        let chrome = TableChrome::new(self.row_menu);
        let viewport_fill = self.viewport_fill;
        let viewport_width = self.viewport_width;
        let empty = self.empty;
        let root_state = state.clone();
        let menu_chrome = chrome.clone();
        let menu_state = state.clone();
        let empty_state = state.clone();
        let prefix = if self.select { select::WIDTH } else { 0.0 };
        let hscroll = hscroll::bind(&state, viewport_width, prefix);
        let body = body_view(
            state.clone(),
            chrome.clone(),
            BodySpec {
                row_height: self.row_height,
                viewport_height: self.viewport_height,
                viewport_width,
                viewport_fill,
                select: self.select,
            },
            hscroll,
        );
        let root = container()
            .role(Role::Grid)
            .row_count(derived(move || root_state.visible().get().len() + 1))
            .column_count(state.columns().len() + usize::from(self.select))
            .class("border border-gray-6 rounded-md bg-surface")
            .layout(
                self.layout_overrides
                    .apply(root_layout(viewport_fill, viewport_width)),
            )
            .child(header_row(
                &state,
                self.header_height,
                &chrome,
                hscroll,
                self.select,
            ))
            .child(body)
            .child(container().children_reactive(move || {
                if empty_state.visible().get().is_empty() {
                    vec![empty_slot(empty.as_ref())]
                } else {
                    Vec::new()
                }
            }))
            .child(hscroll.scrollbar())
            .child(context_overlay(menu_state, menu_chrome));
        wire_hscroll(apply_mutators(root, self.mutators), hscroll)
    }
}

/// The grid root layout: fill the host pane, or size to the explicit
/// viewport width.
fn root_layout(viewport_fill: bool, viewport_width: Option<f32>) -> LayoutStyle {
    let mut layout = LayoutStyle::flex_column()
        .fill_width()
        .overflow_x(Overflow::Hidden);
    if viewport_fill {
        layout = layout.grow(1.0).fill_height();
    } else if let Some(width) = viewport_width {
        layout = layout.width(width);
    }
    layout
}

/// Body viewport sizing resolved from the builder.
#[derive(Clone, Copy)]
struct BodySpec {
    row_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    viewport_fill: bool,
    select: bool,
}

/// The windowed body list, wired into the table's horizontal scroll.
fn body_view<Row: 'static>(
    state: TableState<Row>,
    chrome: TableChrome<Row>,
    spec: BodySpec,
    hs: hscroll::HScroll,
) -> impl IntoNode {
    let body = virtual_list(
        state.visible(),
        body_row(
            state,
            RowSpec {
                row_height: spec.row_height,
                select: spec.select,
            },
            chrome,
            hs,
        ),
    )
    .key_by(|_, id| u64::from(*id))
    .row_height(spec.row_height)
    .scroll_x_bind(hs.scroll_x, hs.max, hs.viewport);
    if spec.viewport_fill {
        body.viewport_fill()
    } else {
        let body = body.viewport_height(spec.viewport_height);
        match spec.viewport_width {
            Some(width) => body.viewport_width(width),
            None => body,
        }
    }
}

fn apply_mutators(mut root: NodeBuilder, mutators: Vec<BuilderMutator>) -> NodeBuilder {
    for mutate in mutators {
        root = mutate(root);
    }
    root
}

/// Attach the width probe (when the viewport is measured, not explicit)
/// and the horizontal wheel handler for the header band and root.
fn wire_hscroll(root: NodeBuilder, hs: hscroll::HScroll) -> NodeBuilder {
    let root = match hs.probe {
        Some(probe) => root.child(hscroll::probe(probe)),
        None => root,
    };
    root.on_scroll(move |ev| {
        wheel_x(ev, hs.scroll_x, hs.viewport.peek(), hs.max.peek());
    })
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
