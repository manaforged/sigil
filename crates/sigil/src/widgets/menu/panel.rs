use std::rc::Rc;

use crate::events::prelude::{MouseButton, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role};
use crate::layout::PopupSide;
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutStyle, Position};
use crate::reactive::{Signal, derived_eq, signal};
use crate::style::{Metrics, Size};

use super::super::listbox::{ListModel, listbox};
use super::super::popup::{self, ItemSlots, item_row};
use super::builder::MenuItem;

#[derive(Clone)]
pub(super) enum Row {
    Item(MenuItem),
    Separator,
}

type Commit = Rc<dyn Fn(usize)>;

struct MenuModel {
    rows: Rc<Vec<Row>>,
    cursor: Signal<usize>,
    commit: Commit,
    close: Rc<dyn Fn()>,
}

impl ListModel for MenuModel {
    fn cursor(&self) -> Signal<usize> {
        self.cursor
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn enabled(&self, index: usize) -> bool {
        matches!(self.rows.get(index), Some(Row::Item(item)) if !item.disabled)
    }

    fn label(&self, index: usize) -> Option<String> {
        match self.rows.get(index) {
            Some(Row::Item(item)) => Some(item.label.clone()),
            _ => None,
        }
    }

    fn commit(&self, index: usize) {
        (self.commit)(index);
    }

    fn close(&self) {
        (self.close)();
    }
}

pub(super) fn panel_height(rows: &[Row]) -> f32 {
    let item = Metrics::current().control_height(Size::Default);
    popup::panel_height(rows.iter().map(|row| match row {
        Row::Item(_) => item,
        Row::Separator => popup::separator_height(),
    }))
}

pub(super) fn panel_layout(top: f32, left: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_top(top)
        .inset_left(left)
        .width(width)
        .height(height)
        .padding(Metrics::current().overlay_padding())
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
        .justify_content(JustifyContent::Start)
}

pub(super) fn menu_panel(rows: Vec<Row>, close: Rc<dyn Fn()>, side: PopupSide) -> NodeBuilder {
    let rows = Rc::new(rows);
    let cursor = signal(usize::MAX);
    let commit = commit_fn(Rc::clone(&rows), Rc::clone(&close));
    let mut panel = popup::popup(Role::Menu, side).focusable(true);
    for (index, row) in rows.iter().enumerate() {
        panel = match row {
            Row::Item(item) => panel.child(build_item(item, index, cursor, Rc::clone(&commit))),
            Row::Separator => panel.child(popup::separator()),
        };
    }
    listbox(
        panel,
        MenuModel {
            rows,
            cursor,
            commit,
            close,
        },
    )
}

fn commit_fn(rows: Rc<Vec<Row>>, close: Rc<dyn Fn()>) -> Commit {
    Rc::new(move |index| {
        let Some(Row::Item(item)) = rows.get(index) else {
            return;
        };
        if item.disabled {
            return;
        }
        if let Some(select) = &item.select {
            select();
        }
        close();
    })
}

fn build_item(item: &MenuItem, index: usize, cursor: Signal<usize>, commit: Commit) -> NodeBuilder {
    let disabled = item.disabled;
    let slots = ItemSlots {
        shortcut: item.shortcut.clone(),
        chevron: item.submenu_indicator,
        disabled,
        ..ItemSlots::new(item.label.clone())
    };
    let highlight = derived_eq(move || cursor.get() == index);
    item_row(slots, highlight)
        .role(Role::MenuItem)
        .hittable(true)
        .pointer_enter(move |_| {
            if !disabled {
                cursor.set(index);
            }
        })
        .pointer_leave(move |_| {
            if cursor.peek() == index {
                cursor.set(usize::MAX);
            }
        })
        .pointer_up(move |event| {
            if event.button != Some(MouseButton::Left) || disabled {
                return;
            }
            event.stop_propagation();
            commit(index);
        })
}
