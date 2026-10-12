use crate::reactive::Reactive;
use std::rc::Rc;

use crate::events::NodeEventsExt;
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{LayoutBuilderExt, LayoutReactiveExt, Overflow, Position};
use crate::layout::{LayoutStyle, snap};
use crate::reactive::{Signal, derived};
use crate::style::DISABLED_OPACITY;
use crate::style::{BORDER_WIDTH, PaintTarget, Radius, Style, StyleAtom, StyleSlot, Token, space};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::menu::MenuItem;
use super::super::scroll::{WheelAxis, wheel};
use super::super::virtual_list::virtual_list;
use super::chrome::{RowActivate, RowMenu, TableChrome, context_overlay};
use super::density::Density;
use super::header::{HeaderSpec, header_row};
use super::hscroll;
use super::row::{RowSpec, body_row};
use super::select;
use super::state::TableState;

static DIMMED: &[StyleAtom] = &[StyleAtom::Opacity(DISABLED_OPACITY)];
static FRAMED: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::BorderStrong.border(),
    Radius::Md.atom(),
    Token::Surface.bg(),
];
static UNFRAMED: &[StyleAtom] = &[Token::Surface.bg()];
const DEFAULT_HEADER_HEIGHT: f32 = space(9);

pub struct TableBuilder<Row: 'static> {
    state: TableState<Row>,
    row_height: f32,
    header_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    viewport_fill: bool,
    row_menu: Option<RowMenu<Row>>,
    row_activate: Option<RowActivate<Row>>,
    empty: Option<Rc<dyn Fn() -> NodeBuilder>>,
    select: bool,
    density: Density,
    striped: bool,
    frame: &'static [StyleAtom],
    scroll_y: Option<Signal<f32>>,
    disabled: Option<Signal<bool>>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn table<Row: 'static>(state: TableState<Row>) -> TableBuilder<Row> {
    TableBuilder {
        state,
        row_height: space(9),
        header_height: DEFAULT_HEADER_HEIGHT,
        viewport_height: space(120),
        viewport_width: None,
        viewport_fill: false,
        row_menu: None,
        row_activate: None,
        empty: None,
        select: false,
        density: Density::Default,
        striped: false,
        frame: FRAMED,
        scroll_y: None,
        disabled: None,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl<Row: 'static> TableBuilder<Row> {
    #[must_use]
    pub fn row_height(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.row_height = length.into_px();
        self
    }

    #[must_use]
    pub const fn header_height(mut self, px: f32) -> Self {
        self.header_height = px;
        self
    }

    #[must_use]
    pub fn viewport_height(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.viewport_height = length.into_px();
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
    pub fn row_activate(mut self, activate: impl Fn(&Row) + 'static) -> Self {
        self.row_activate = Some(Rc::new(activate));
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
    pub const fn framed(mut self, framed: bool) -> Self {
        self.frame = if framed { FRAMED } else { UNFRAMED };
        self
    }

    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
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

    fn into_node_builder(self) -> NodeBuilder {
        let state = self.state;
        let chrome = TableChrome::new(self.row_menu, self.row_activate);
        let pad_x = self.density.pad_x();
        let prefix = if self.select { select::WIDTH } else { 0.0 };
        let hscroll = hscroll::bind(&state, self.viewport_width, prefix);
        let row = RowSpec {
            row_height: snap::line(self.row_height),
            select: self.select,
            pad_x,
            striped: self.striped,
            disabled: self.disabled,
        };
        let viewport = (!self.viewport_fill).then_some(self.viewport_height);
        let body = body_view(
            state.clone(),
            chrome.clone(),
            row,
            hscroll,
            (viewport, self.scroll_y),
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
            .row_count(derived(move || root_state.visible().get().len() + 1))
            .column_count(state.columns().len() + usize::from(self.select))
            .class(self.frame)
            .class(self.density.text())
            .with(
                self.layout_overrides
                    .apply(root_layout(self.viewport_fill, self.viewport_width)),
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

fn root_layout(viewport_fill: bool, viewport_width: Option<f32>) -> LayoutStyle {
    let layout = LayoutStyle::flex_column()
        .fill_width()
        .overflow_x(Overflow::Hidden);
    match viewport_width {
        _ if viewport_fill => layout.grow(1.0).fill_height(),
        Some(width) => layout.width(width),
        None => layout,
    }
}

fn body_view<Row: 'static>(
    state: TableState<Row>,
    chrome: TableChrome<Row>,
    row: RowSpec,
    hs: hscroll::HScroll,
    (viewport, scroll_y): (Option<f32>, Option<Signal<f32>>),
) -> impl IntoNode {
    let body = virtual_list(state.visible(), body_row(state, row, chrome, hs))
        .key_by(|_, id| u64::from(*id))
        .row_height(row.row_height)
        .scroll_x_bind(hs.scroll_x, hs.max, hs.viewport);
    let body = match scroll_y {
        Some(scroll_y) => body.scroll_bind(scroll_y),
        None => body,
    };
    match viewport {
        Some(height) => body.viewport_height(height).shrink_to_content(),
        None => body.viewport_fill(),
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
    let root = if style.is_empty() {
        root
    } else {
        root.style(style)
    };
    let root = match hs.probe {
        Some(probe) => root.child(hscroll::probe(probe)),
        None => root,
    };
    root.on_scroll(move |ev| {
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
                .padding(space(6))
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

impl<Row: 'static> IntoNode for TableBuilder<Row> {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
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

impl<Row: 'static> PaintTarget for TableBuilder<Row> {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}
