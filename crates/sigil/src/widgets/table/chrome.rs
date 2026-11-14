use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string;
use std::rc::Rc;

use crate::geometry::Point;
use crate::kernel::{NodeBuilder, container};
use crate::layout::{LayoutStyle, Position};
use crate::reactive::{Owner, Signal, effect, memo, signal};
use crate::style::{ControlSize, Metrics};
use crate::widgets::Widget;

use super::super::menu::PANEL_INSET;
use super::super::menu::{MenuItem, menu};
use super::edit::CellEditor;
use super::state::TableState;
use crate::kernel::SortDirection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContextKind {
    Header(&'static str),
    Row(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OpenContext {
    kind: ContextKind,
    pos: (f32, f32),
}

pub(super) type RowMenu<Row> = Rc<dyn Fn(&Row) -> Vec<MenuItem>>;
pub(super) type RowActivate<Row> = Rc<dyn Fn(&Row)>;

pub(super) struct TableChrome<Row: 'static> {
    pub context: Signal<Option<OpenContext>>,
    pub bounds: Signal<crate::geometry::Rect>,
    pub row_menu: Option<RowMenu<Row>>,
    pub row_activate: Option<RowActivate<Row>>,
    pub host: Option<Owner>,
    pub edit: CellEditor,
}

impl<Row: 'static> Clone for TableChrome<Row> {
    fn clone(&self) -> Self {
        Self {
            context: self.context,
            bounds: self.bounds,
            row_menu: self.row_menu.clone(),
            row_activate: self.row_activate.clone(),
            host: self.host,
            edit: self.edit,
        }
    }
}

impl<Row: 'static> TableChrome<Row> {
    pub(super) fn new(
        row_menu: Option<RowMenu<Row>>,
        row_activate: Option<RowActivate<Row>>,
    ) -> Self {
        Self {
            context: signal(None),
            bounds: signal(crate::geometry::Rect::default()),
            row_menu,
            row_activate,
            host: Owner::current(),
            edit: CellEditor::new(),
        }
    }

    pub(super) fn activate(&self, state: &TableState<Row>, row: u32) {
        let Some(activate) = &self.row_activate else {
            return;
        };
        if let Some(value) = state.rows().peek().get(row as usize) {
            activate(value);
        }
    }

    pub(super) fn open(&self, kind: ContextKind, pos: impl Into<(f32, f32)>) {
        self.context.set(Some(OpenContext {
            kind,
            pos: pos.into(),
        }));
    }
}

pub(super) fn context_overlay<Row: 'static>(
    state: TableState<Row>,
    chrome: TableChrome<Row>,
) -> NodeBuilder {
    let visible = memo(move || chrome.context.get().is_some());
    container()
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_left(0.0)
                .inset_top(0.0)
                .fill_width()
                .fill_height(),
        )
        .visible(visible)
        .children_reactive(move || build_context_menu(&state, &chrome))
}

fn build_context_menu<Row: 'static>(
    state: &TableState<Row>,
    chrome: &TableChrome<Row>,
) -> Vec<crate::widgets::MenuBuilder> {
    let Some(ctx) = chrome.context.get() else {
        return Vec::new();
    };
    let (x, y) = ctx.pos;
    let items = match ctx.kind {
        ContextKind::Header(id) => header_menu(state, id),
        ContextKind::Row(index) => row_menu_items(state, chrome, index),
    };
    if items.is_empty() {
        return Vec::new();
    }
    let bounds = chrome.bounds.get();
    let (left, top, width, height) = (
        bounds.origin.x,
        bounds.origin.y,
        bounds.width,
        bounds.height,
    );
    let menu_width = width.min(200.0);
    let item_height = Metrics::current().control_height(ControlSize::Default);
    let menu_height = items
        .iter()
        .fold(PANEL_INSET * 2.0, |total, _| total + item_height);
    let x = (x - left).clamp(0.0, (width - menu_width).max(0.0));
    let y = (y - top).clamp(0.0, (height - menu_height).max(0.0));
    let menu_open = signal(true);
    let context = chrome.context;
    effect(move || {
        if !menu_open.get() {
            context.set(None);
        }
    });
    let mut panel = menu(menu_open);
    if let Some(host) = chrome.host {
        panel = panel.host(host);
    }
    let mut panel = panel
        .accessible(theme_string(StringKey::TableMenu))
        .anchor(Point::new(x, y))
        .width(menu_width);
    for item in items {
        panel = panel.item(item);
    }
    vec![panel]
}

fn header_menu<Row: 'static>(state: &TableState<Row>, id: &'static str) -> Vec<MenuItem> {
    let item = |key: StringKey, action: fn(&TableState<Row>, &'static str)| {
        let state = state.clone();
        MenuItem::new(theme_string(key)).activate(move || action(&state, id))
    };
    let mut items = Vec::new();
    if state.is_sortable(id) {
        items.push(item(StringKey::SortAscending, |table, column| {
            table.sort(column, SortDirection::Ascending);
        }));
        items.push(item(StringKey::SortDescending, |table, column| {
            table.sort(column, SortDirection::Descending);
        }));
    }
    items.push(item(StringKey::HideColumn, TableState::hide_column));
    items.push(item(StringKey::Autosize, TableState::clear_column_width));
    let count = state.column_layout().peek().order.len();
    let visual = state
        .visible_columns()
        .into_iter()
        .position(|(_, col)| col.id() == id);
    if let Some(from) = visual {
        let shift = |key: StringKey, to: usize| {
            let state = state.clone();
            MenuItem::new(theme_string(key)).activate(move || state.move_column(from, to))
        };
        if from > 0 {
            items.push(shift(StringKey::MoveLeft, from - 1));
        }
        if from + 1 < count {
            items.push(shift(StringKey::MoveRight, from + 1));
        }
    }
    items.push(item(StringKey::ShowAllColumns, |table, _| {
        table.show_all_columns();
    }));
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
