use crate::layout::Space;
use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{LayoutStyle, snap};
use crate::layout::{Overflow, Position};
use crate::reactive::{Signal, memo};
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom, Token, space};
use crate::widgets::{Layout, Widget};

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
const DEFAULT_HEADER_HEIGHT: f32 = space(9);

struct BodySpec {
    viewport: Option<f32>,
    scroll_y: Option<Signal<f32>>,
    end_reached: Option<EndReached>,
}

/// Builder of a table.
#[must_use]
pub struct TableBuilder<Row: 'static> {
    state: TableState<Row>,
    row_height: f32,
    header_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    row_menu: Option<RowMenu<Row>>,
    row_activate: Option<RowActivate<Row>>,
    empty: Option<Rc<dyn Fn() -> NodeBuilder>>,
    select: bool,
    density: Density,
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
        row_height: space(9),
        header_height: DEFAULT_HEADER_HEIGHT,
        viewport_height: space(120),
        viewport_width: None,
        row_menu: None,
        row_activate: None,
        empty: None,
        select: false,
        density: Density::Default,
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
    /// Sets the row height.
    pub fn row_height(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.row_height = length.into_px();
        self
    }

    /// Sets the header height.
    pub const fn header_height(mut self, px: f32) -> Self {
        self.header_height = px;
        self
    }

    /// Sets the viewport height.
    pub fn viewport_height(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.viewport_height = length.into_px();
        self
    }

    /// Sets the viewport width.
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    /// Sets the row menu.
    pub fn row_menu(mut self, build: impl Fn(&Row) -> Vec<MenuItem> + 'static) -> Self {
        self.row_menu = Some(Rc::new(build));
        self
    }

    /// Sets the row activate.
    pub fn row_activate(mut self, activate: impl Fn(&Row) + 'static) -> Self {
        self.row_activate = Some(Rc::new(activate));
        self
    }

    /// Sets the density.
    pub const fn density(mut self, density: Density) -> Self {
        self.row_height = density.row();
        self.header_height = density.header();
        self.density = density;
        self
    }

    /// Applies the striped option.
    pub const fn striped(mut self) -> Self {
        self.striped = true;
        self
    }

    /// Sets the framed.
    pub const fn framed(mut self, framed: bool) -> Self {
        self.frame = if framed { FRAMED } else { UNFRAMED };
        self
    }

    /// Sets the scroll bind.
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Runs `f` once each time the user scrolls within `rows` rows of the end, for load-more paging.
    pub fn end_reached(mut self, rows: usize, f: impl Fn() + 'static) -> Self {
        self.end_reached = Some(EndReached::new(rows, f));
        self
    }

    /// Sets the disabled.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the empty.
    pub fn empty(mut self, build: impl Fn() -> NodeBuilder + 'static) -> Self {
        self.empty = Some(Rc::new(build));
        self
    }

    /// Applies the selectable checkboxes option.
    pub const fn selectable_checkboxes(mut self) -> Self {
        self.select = true;
        self
    }

    fn row_spec(&self) -> RowSpec {
        RowSpec {
            row_height: snap::line(self.row_height),
            select: self.select,
            pad_x: self.density.pad_x(),
            striped: self.striped,
            disabled: self.disabled,
        }
    }

    fn root(self) -> NodeBuilder {
        let row = self.row_spec();
        let state = self.state;
        let chrome = TableChrome::new(self.row_menu, self.row_activate);
        let pad_x = self.density.pad_x();
        let prefix = if self.select { select::WIDTH } else { 0.0 };
        let hscroll = hscroll::bind(&state, self.viewport_width, prefix);
        let fill = self.layout_overrides.fills_height();
        let viewport = (!fill).then_some(self.viewport_height);
        let body = body_view(
            state.clone(),
            chrome.clone(),
            row,
            hscroll,
            BodySpec {
                viewport,
                scroll_y: self.scroll_y,
                end_reached: self.end_reached,
            },
        );
        let header = HeaderSpec {
            height: snap::line(self.header_height),
            select: self.select,
            pad_x,
            disabled: self.disabled,
        };
        let root_state = state.clone();
        let root = container()
            .bounds_reactive(chrome.bounds)
            .role(Role::Grid)
            .row_count(memo(move || root_state.visible().get().len() + 1))
            .column_count(state.columns().len() + usize::from(self.select))
            .class(self.frame)
            .class(self.density.text())
            .with(
                self.layout_overrides
                    .apply(root_layout(fill, self.viewport_width)),
            )
            .child(header_row(&state, header, &chrome, hscroll))
            .child(body)
            .child(empty_overlay(
                state.clone(),
                self.empty,
                snap::ruled(self.header_height),
            ))
            .child(hscroll.scrollbar())
            .child(context_overlay(state, chrome));
        finish(root, self.disabled, self.style_accumulator, hscroll)
    }
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
        .scroll_x_bind(hs.scroll_x, hs.max, hs.viewport);
    let body = match spec.scroll_y {
        Some(scroll_y) => body.scroll_bind(scroll_y),
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
        Some(height) => body.viewport_height(height).shrink_to_content(),
        None => body.fill(),
    }
}

fn finish(
    root: NodeBuilder,
    disabled: Option<Signal<bool>>,
    style: Style,
    hs: hscroll::HScroll,
) -> NodeBuilder {
    let root = match disabled {
        Some(disabled) => root.style_reactive(move || if disabled.get() { DIMMED } else { &[] }),
        None => root,
    };
    let root = caller_style(root, style);
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
                .class(Token::Gray11.text()),
        )
}

builder_impls!([Row: 'static] TableBuilder<Row>: paint(style_accumulator), widget, node);

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
