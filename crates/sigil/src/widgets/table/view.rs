use crate::layout::Space;
use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{LayoutStyle, snap};
use crate::layout::{Overflow, Position};
use crate::reactive::{Signal, memo};
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom, Token, Tone, space};
use crate::widgets::Widget;

use super::super::layout_ext::LayoutOverrides;
use super::super::menu::MenuItem;
use super::super::scroll::{WheelAxis, wheel};
use super::super::shared::{DIMMED, caller_style};
use super::super::virtual_list::{EndReached, virtual_list};
use super::chrome::{RowActivate, RowMenu, TableChrome, context_overlay};
use super::columns::TableColumn;
use super::density::Density;
use super::header::{HeaderSpec, header_row};
use super::hscroll;
use super::row::{RowSpec, body_row};
use super::select;
use super::state::TableState;

static FRAMED: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::BorderStrong.border(),
    Radius::Md.atom(),
    Token::Surface.background(),
];
static UNFRAMED: &[StyleAtom] = &[Token::Surface.background()];

struct BodySpec {
    viewport: Option<f32>,
    scroll_y: Option<Signal<f32>>,
    end_reached: Option<EndReached>,
}

/// Builder of a table.
#[must_use]
pub struct TableBuilder<Row: 'static> {
    state: TableState<Row>,
    row_height: Option<f32>,
    header_height: Option<f32>,
    viewport_height: f32,
    viewport_width: Option<f32>,
    row_menu: Option<RowMenu<Row>>,
    row_activate: Option<RowActivate<Row>>,
    empty: Option<Rc<dyn Fn() -> NodeBuilder>>,
    select: bool,
    density: Option<Signal<Density>>,
    striped: bool,
    frame: &'static [StyleAtom],
    scroll_y: Option<Signal<f32>>,
    end_reached: Option<EndReached>,
    disabled: Option<Signal<bool>>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl<Row: 'static> core::fmt::Debug for TableBuilder<Row> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TableBuilder")
            .field("state", &self.state)
            .field("row_height", &self.row_height)
            .field("header_height", &self.header_height)
            .field("viewport_height", &self.viewport_height)
            .field("select", &self.select)
            .field("density", &self.density)
            .field("striped", &self.striped)
            .finish_non_exhaustive()
    }
}

/// A sortable, filterable table.
pub fn table<Row: 'static>(state: TableState<Row>) -> TableBuilder<Row> {
    TableBuilder {
        state,
        row_height: None,
        header_height: None,
        viewport_height: space(120),
        viewport_width: None,
        row_menu: None,
        row_activate: None,
        empty: None,
        select: false,
        density: None,
        striped: false,
        frame: FRAMED,
        scroll_y: None,
        end_reached: None,
        disabled: None,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl<Row: 'static> TableBuilder<Row> {
    /// Sets the height of each body row.
    pub fn row_height(mut self, length: impl Into<crate::layout::Length>) -> Self {
        self.row_height = crate::layout::positive_px(length);
        self
    }

    /// Sets the height of the header row in px.
    pub const fn header_height(mut self, px: f32) -> Self {
        self.header_height = Some(px);
        self
    }

    /// Sets the largest height of the body when the table does not fill its parent.
    pub fn viewport_height(mut self, length: impl Into<crate::layout::Length>) -> Self {
        self.viewport_height = crate::layout::px_or_zero(length);
        self
    }

    /// Sets the width of the table in px.
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    /// Shows the items that `build` returns in a context menu when the user right-clicks a row.
    pub fn row_menu(mut self, build: impl Fn(&Row) -> Vec<MenuItem> + 'static) -> Self {
        self.row_menu = Some(Rc::new(build));
        self
    }

    /// Runs `activate` with the row when the user clicks it or presses Enter on it.
    pub fn row_activate(mut self, activate: impl Fn(&Row) + 'static) -> Self {
        self.row_activate = Some(Rc::new(activate));
        self
    }

    /// Sets the row height, header height, cell padding, and text size from `density`.
    pub fn density(mut self, density: impl Reactive<Density>) -> Self {
        self.density = Some(density.into_signal());
        self
    }

    /// Shades every second visible row.
    pub const fn striped(mut self) -> Self {
        self.striped = true;
        self
    }

    /// Draws a border and rounded corners around the table when `framed` is true, the default.
    pub const fn framed(mut self, framed: bool) -> Self {
        self.frame = if framed { FRAMED } else { UNFRAMED };
        self
    }

    /// Binds the vertical scroll offset of the body in px to `scroll_y`.
    pub const fn scroll_y(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Runs `f` once each time the user scrolls within `rows` rows of the end, for load-more paging.
    pub fn end_reached(mut self, rows: usize, f: impl Fn() + 'static) -> Self {
        self.end_reached = Some(EndReached::new(rows, f));
        self
    }

    /// Dims the table and blocks row selection and activation while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Shows the node that `build` returns when no row is visible; the default text is "No rows".
    pub fn empty(mut self, build: impl Fn() -> NodeBuilder + 'static) -> Self {
        self.empty = Some(Rc::new(build));
        self
    }

    /// Adds a first column with a checkbox in the header and in each row.
    pub const fn selectable_checkboxes(mut self) -> Self {
        self.select = true;
        self
    }

    fn root(self) -> NodeBuilder {
        let fill = self.layout_overrides.fills_height();
        let layout = self
            .layout_overrides
            .apply(root_layout(fill, self.viewport_width));
        let density = self.density;
        let style = self.style_accumulator;
        let config = TableConfig {
            state: self.state,
            row_height: self.row_height,
            header_height: self.header_height,
            viewport_height: self.viewport_height,
            viewport_width: self.viewport_width,
            row_menu: self.row_menu,
            row_activate: self.row_activate,
            empty: self.empty,
            select: self.select,
            striped: self.striped,
            frame: self.frame,
            scroll_y: self.scroll_y,
            end_reached: self.end_reached,
            disabled: self.disabled,
        };
        let node = match density {
            Some(density_sig) => container().with(layout).children_reactive(move || {
                let d = density_sig.get();
                vec![build_table_node(
                    &config,
                    d,
                    fill,
                    root_layout(fill, config.viewport_width),
                )]
            }),
            None => build_table_node(&config, Density::Default, fill, layout),
        };
        caller_style(node, style)
    }
}

struct TableConfig<Row: 'static> {
    state: TableState<Row>,
    row_height: Option<f32>,
    header_height: Option<f32>,
    viewport_height: f32,
    viewport_width: Option<f32>,
    row_menu: Option<RowMenu<Row>>,
    row_activate: Option<RowActivate<Row>>,
    empty: Option<Rc<dyn Fn() -> NodeBuilder>>,
    select: bool,
    striped: bool,
    frame: &'static [StyleAtom],
    scroll_y: Option<Signal<f32>>,
    end_reached: Option<EndReached>,
    disabled: Option<Signal<bool>>,
}

fn table_specs<Row: 'static>(
    config: &TableConfig<Row>,
    density: Density,
) -> (RowSpec, HeaderSpec, f32) {
    let row_height = config.row_height.unwrap_or_else(|| density.row());
    let header_height = config.header_height.unwrap_or_else(|| density.header());
    let pad_x = density.pad_x();
    let row = RowSpec {
        row_height: snap::line(row_height),
        select: config.select,
        pad_x,
        striped: config.striped,
        disabled: config.disabled,
    };
    let header = HeaderSpec {
        height: snap::line(header_height),
        select: config.select,
        pad_x,
        disabled: config.disabled,
    };
    (row, header, header_height)
}

fn build_table_node<Row: 'static>(
    config: &TableConfig<Row>,
    density: Density,
    fill: bool,
    layout: LayoutStyle,
) -> NodeBuilder {
    let (row, header, header_height) = table_specs(config, density);
    let state = config.state.clone();
    let chrome = TableChrome::new(config.row_menu.clone(), config.row_activate.clone());
    let hscroll = hscroll::bind(
        &state,
        config.viewport_width,
        if config.select { select::WIDTH } else { 0.0 },
    );
    let body = body_view(
        state.clone(),
        chrome.clone(),
        row,
        hscroll,
        BodySpec {
            viewport: (!fill).then_some(config.viewport_height),
            scroll_y: config.scroll_y,
            end_reached: config.end_reached.clone(),
        },
    );
    let root_state = state.clone();
    let root = container()
        .track_bounds(chrome.bounds)
        .role(Role::Grid)
        .row_count(memo(move || root_state.visible().get().len() + 1))
        .column_count(state.columns().len() + usize::from(config.select))
        .class(config.frame)
        .class(density.text())
        .with(layout)
        .child(header_row(&state, header, &chrome, hscroll))
        .child(body)
        .child(empty_overlay(
            state.clone(),
            config.empty.clone(),
            snap::ruled(header_height),
        ))
        .child(hscroll.scrollbar())
        .child(context_overlay(state, chrome));
    finish(root, config.disabled, hscroll)
}

fn empty_overlay<Row: 'static>(
    state: TableState<Row>,
    empty: Option<Rc<dyn Fn() -> NodeBuilder>>,
    header_height: f32,
) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .inset_top(header_height)
                .inset_left(0.0)
                .inset_right(0.0),
        )
        .children_reactive(move || {
            if state.visible().get().is_empty() {
                vec![empty.as_ref().map_or_else(default_empty, |build| build())]
            } else {
                Vec::new()
            }
        })
}

fn root_layout(fill: bool, viewport_width: Option<f32>) -> LayoutStyle {
    let layout = LayoutStyle::flex_column()
        .fill_width()
        .min_size(0.0, 0.0)
        .overflow_x(Overflow::Hidden);
    match viewport_width {
        _ if fill => layout.grow(1.0),
        Some(width) => layout.width(width),
        None => layout,
    }
}

fn body_view<Row: 'static>(
    state: TableState<Row>,
    chrome: TableChrome<Row>,
    row: RowSpec,
    hs: hscroll::HScroll,
    spec: BodySpec,
) -> impl IntoNode {
    let follow = state
        .columns()
        .iter()
        .any(TableColumn::is_editable)
        .then_some(chrome.edit.follow);
    let body = virtual_list(state.visible(), body_row(state, row, chrome, hs))
        .key_by(|_, id| u64::from(*id))
        .row_height(row.row_height)
        .scroll_x(hs.scroll_x, hs.max, hs.viewport);
    let body = match spec.scroll_y {
        Some(scroll_y) => body.scroll_y(scroll_y),
        None => body,
    };
    let body = match follow {
        Some(follow) => body.follow_row(follow),
        None => body,
    };
    let body = match spec.end_reached {
        Some(hook) => body.end_hook(hook),
        None => body,
    };
    match spec.viewport {
        Some(height) => body.viewport_height(height).fit(),
        None => body.fill(),
    }
}

fn finish(root: NodeBuilder, disabled: Option<Signal<bool>>, hs: hscroll::HScroll) -> NodeBuilder {
    let root = match disabled {
        Some(disabled) => root.style_reactive(move || if disabled.get() { DIMMED } else { &[] }),
        None => root,
    };
    let root = match hs.probe {
        Some(probe) => root.child(hscroll::probe(probe)),
        None => root,
    };
    root.scroll(move |ev| {
        let x = WheelAxis {
            offset: hs.scroll_x,
            viewport: hs.viewport.peek(),
            max: hs.max.peek(),
        };
        wheel(ev, None, Some(x));
    })
}

fn default_empty() -> NodeBuilder {
    use crate::kernel::text;
    use crate::layout::{AlignItems, JustifyContent};
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .padding(Space::Xl)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .child(
            text("No rows")
                .nowrap()
                .class("text-sm")
                .class(Tone::Neutral.step(11).text()),
        )
}

builder_impls!([Row: 'static] TableBuilder<Row>: paint(style_accumulator), widget, node);

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
