use std::rc::Rc;

use crate::events::prelude::NodeEventsExt;
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::LayoutStyle;
use crate::layout::prelude::{LayoutBuilderExt, LayoutReactiveExt, Overflow, Position};
use crate::reactive::{Signal, derived};
use crate::style::{ClassSource, NodeStyleExt};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::menu::MenuItem;
use super::super::scroll::wheel_x;
use super::super::shared::BuilderMutator;
use super::super::virtual_list::virtual_list;
use super::chrome::{RowMenu, TableChrome, context_overlay};
use super::header::{HeaderSpec, header_row};
use super::hscroll;
use super::row::{RowSpec, body_row};
use super::select;
use super::state::TableState;

const DEFAULT_ROW_HEIGHT: f32 = 36.0;
const DEFAULT_HEADER_HEIGHT: f32 = 36.0;
const DEFAULT_VIEWPORT_HEIGHT: f32 = 480.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Density {
    Grid,
    Compact,
    #[default]
    Default,
}

impl Density {
    const fn row(self) -> f32 {
        match self {
            Self::Grid => 24.0,
            Self::Compact => 28.0,
            Self::Default => 36.0,
        }
    }

    const fn header(self) -> f32 {
        match self {
            Self::Grid => 26.0,
            Self::Compact => 32.0,
            Self::Default => 36.0,
        }
    }

    const fn pad_x(self) -> f32 {
        match self {
            Self::Grid => 6.0,
            Self::Compact => 8.0,
            Self::Default => 12.0,
        }
    }

    const fn text(self) -> &'static str {
        match self {
            Self::Grid => "text-xs",
            Self::Compact | Self::Default => "text-sm",
        }
    }
}

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
    density: Density,
    striped: bool,
    scroll_y: Option<Signal<f32>>,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
}

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
        density: Density::Default,
        striped: false,
        scroll_y: None,
        mutators: Vec::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl<Row: 'static> TableBuilder<Row> {
    #[must_use]
    pub const fn row_height(mut self, px: f32) -> Self {
        self.row_height = px;
        self
    }

    #[must_use]
    pub const fn header_height(mut self, px: f32) -> Self {
        self.header_height = px;
        self
    }

    #[must_use]
    pub const fn viewport_height(mut self, px: f32) -> Self {
        self.viewport_height = px;
        self
    }

    #[must_use]
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    #[must_use]
    pub const fn viewport_fill(mut self) -> Self {
        self.viewport_fill = true;
        self
    }

    #[must_use]
    pub fn row_menu(mut self, build: impl Fn(&Row) -> Vec<MenuItem> + 'static) -> Self {
        self.row_menu = Some(Rc::new(build));
        self
    }

    #[must_use]
    pub const fn density(mut self, density: Density) -> Self {
        self.row_height = density.row();
        self.header_height = density.header();
        self.density = density;
        self
    }

    #[must_use]
    pub const fn striped(mut self) -> Self {
        self.striped = true;
        self
    }

    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    #[must_use]
    pub fn empty(mut self, build: impl Fn() -> NodeBuilder + 'static) -> Self {
        self.empty = Some(Rc::new(build));
        self
    }

    #[must_use]
    pub const fn selectable_checkboxes(mut self) -> Self {
        self.select = true;
        self
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    const fn body_spec(&self, pad_x: f32) -> BodySpec {
        BodySpec {
            row_height: self.row_height,
            viewport_height: self.viewport_height,
            viewport_fill: self.viewport_fill,
            select: self.select,
            pad_x,
            striped: self.striped,
            scroll_y: self.scroll_y,
        }
    }

    fn into_node_builder(self) -> NodeBuilder {
        let body_spec = self.body_spec(self.density.pad_x());
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
        let pad_x = self.density.pad_x();
        let hscroll = hscroll::bind(&state, viewport_width, prefix);
        let body = body_view(state.clone(), chrome.clone(), body_spec, hscroll);
        let root = container()
            .bounds_reactive(chrome.bounds)
            .role(Role::Grid)
            .row_count(derived(move || root_state.visible().get().len() + 1))
            .column_count(state.columns().len() + usize::from(self.select))
            .class("border border-border-strong rounded-md bg-surface")
            .class(self.density.text())
            .layout(
                self.layout_overrides
                    .apply(root_layout(viewport_fill, viewport_width)),
            )
            .child(header_row(
                &state,
                HeaderSpec {
                    height: self.header_height,
                    select: self.select,
                    pad_x,
                },
                &chrome,
                hscroll,
            ))
            .child(body)
            .child(empty_overlay(empty_state, empty, self.header_height))
            .child(hscroll.scrollbar())
            .child(context_overlay(menu_state, menu_chrome));
        wire_hscroll(apply_mutators(root, self.mutators), hscroll)
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
                vec![empty_slot(empty.as_ref())]
            } else {
                Vec::new()
            }
        })
}

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

#[derive(Clone, Copy)]
struct BodySpec {
    row_height: f32,
    viewport_height: f32,
    viewport_fill: bool,
    select: bool,
    pad_x: f32,
    striped: bool,
    scroll_y: Option<Signal<f32>>,
}

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
                pad_x: spec.pad_x,
                striped: spec.striped,
            },
            chrome,
            hs,
        ),
    )
    .key_by(|_, id| u64::from(*id))
    .row_height(spec.row_height)
    .scroll_x_bind(hs.scroll_x, hs.max, hs.viewport);
    let body = match spec.scroll_y {
        Some(scroll_y) => body.scroll_bind(scroll_y),
        None => body,
    };
    if spec.viewport_fill {
        body.viewport_fill()
    } else {
        body.viewport_height(spec.viewport_height)
    }
}

fn apply_mutators(mut root: NodeBuilder, mutators: Vec<BuilderMutator>) -> NodeBuilder {
    for mutate in mutators {
        root = mutate(root);
    }
    root
}

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
    use crate::layout::prelude::{AlignItems, JustifyContent};
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
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

impl<Row: 'static> WidgetLayoutExt for TableBuilder<Row> {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
