use std::rc::Rc;

use crate::kernel::{NodeBuilder, container};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Owner, OwnerId, Signal, signal};
use crate::style::{Metrics, NodeStyleExt, Size};

use super::super::layout_ext::WidgetLayoutExt;
use super::super::menu::PANEL_INSET;
use super::super::menu::{MenuItem, menu};
use super::state::{SortDir, TableState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContextKind {
    Header(&'static str),
    Row(u32),
}

pub(super) type RowMenu<Row> = Rc<dyn Fn(&Row) -> Vec<MenuItem>>;
pub(super) type RowActivate<Row> = Rc<dyn Fn(&Row)>;

pub(super) struct TableChrome<Row: 'static> {
    pub open: Signal<bool>,
    pub pos: Signal<(f32, f32)>,
    pub bounds: Signal<(f32, f32, f32, f32)>,
    pub kind: Signal<Option<ContextKind>>,
    pub row_menu: Option<RowMenu<Row>>,
    pub row_activate: Option<RowActivate<Row>>,
    pub host: Option<OwnerId>,
}

impl<Row: 'static> Clone for TableChrome<Row> {
    fn clone(&self) -> Self {
        Self {
            open: self.open,
            pos: self.pos,
            bounds: self.bounds,
            kind: self.kind,
            row_menu: self.row_menu.clone(),
            row_activate: self.row_activate.clone(),
            host: self.host,
        }
    }
}

impl<Row: 'static> TableChrome<Row> {
    pub(super) fn new(
        row_menu: Option<RowMenu<Row>>,
        row_activate: Option<RowActivate<Row>>,
    ) -> Self {
        Self {
            open: signal(false),
            pos: signal((0.0, 0.0)),
            bounds: signal((0.0, 0.0, 0.0, 0.0)),
            kind: signal(None),
            row_menu,
            row_activate,
            host: Owner::current(),
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
        self.kind.set(Some(kind));
        self.pos.set(pos.into());
        self.open.set(true);
    }
}

pub(super) fn context_overlay<Row: 'static>(
    state: TableState<Row>,
    chrome: TableChrome<Row>,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_left(0.0)
                .inset_top(0.0)
                .fill_width()
                .fill_height(),
        )
        .visible_when(chrome.open)
        .children_reactive(move || {
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
            let (left, top, width, height) = chrome.bounds.get();
            let menu_width = width.min(200.0);
            let item_height = Metrics::current().control_height(Size::Default);
            let menu_height = items
                .iter()
                .fold(PANEL_INSET * 2.0, |total, _| total + item_height);
            let x = (x - left).clamp(0.0, (width - menu_width).max(0.0));
            let y = (y - top).clamp(0.0, (height - menu_height).max(0.0));
            let mut panel = menu(chrome.open);
            if let Some(host) = chrome.host {
                panel = panel.host(host);
            }
            let mut panel = panel.label("Table menu").anchor(x, y).width(menu_width);
            for item in items {
                panel = panel.item(item);
            }
            vec![panel]
        })
}

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
