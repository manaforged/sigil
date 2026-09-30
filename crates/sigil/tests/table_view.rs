#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
#![allow(clippy::shadow_unrelated)]

#[path = "events_support/input.rs"]
mod input;
#[path = "widget_support/mod.rs"]
mod widget_support;

use input::key_down;
use sigil::events::EventRouter;
use sigil::events::ShellEvent;
use sigil::events::{PhysicalKey, PointerEventInit, ScrollEventInit};
use sigil::geometry::Point;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};
use sigil::widgets::{SortDir, SortKey, TableState};
use widget_support::{compute, rect_colors};

const ROW_H: f32 = 20.0;
const HEADER_H: f32 = 28.0;
const VIEW_H: f32 = 200.0;
const WIDTH: f32 = 200.0;
const VIEWPORT: Size = Size::fixed(640.0, 480.0);

#[derive(Clone)]
struct Item {
    name: String,
    count: i64,
}

fn item(name: &str, count: i64) -> Item {
    Item {
        name: name.to_owned(),
        count,
    }
}

fn demo_rows() -> Vec<Item> {
    vec![
        item("gamma", 30),
        item("alpha", 10),
        item("delta", 40),
        item("beta", 20),
    ]
}

fn demo_state() -> TableState<Item> {
    TableState::new(
        demo_rows(),
        vec![
            TableColumn::new("name", "Name")
                .sort_by(|row: &Item| row.name.clone())
                .filterable(|row: &Item, query: &str| row.name.contains(query))
                .width(ColWidth::Fixed(100.0))
                .cell(|row: &Item| container().child(text(row.name.clone()))),
            TableColumn::new("count", "Count")
                .sort_by(|row: &Item| row.count)
                .width(ColWidth::Fixed(100.0))
                .cell(|row: &Item| container().child(text(row.count.to_string()))),
        ],
    )
}

fn glyph_texts(cx: &Cx, root: NodeId) -> Vec<String> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::GlyphRun(d) => Some(d.run.text.clone()),
            _ => None,
        })
        .collect()
}

const fn pointer(phase: PointerPhase, x: f32, y: f32, modifiers: ModifiersState) -> ShellEvent {
    let (button, buttons) = match phase {
        PointerPhase::Down => (Some(MouseButton::Left), MouseButtons::LEFT),
        _ => (None, MouseButtons::empty()),
    };
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers,
        phase,
    }))
}

fn click(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, x: f32, y: f32) {
    router.route(
        cx,
        layout,
        pointer(PointerPhase::Down, x, y, ModifiersState::empty()),
    );
    router.route(
        cx,
        layout,
        pointer(PointerPhase::Up, x, y, ModifiersState::empty()),
    );
}

fn visible_names(state: &TableState<Item>) -> Vec<String> {
    let rows = state.rows().get();
    state
        .visible()
        .get()
        .iter()
        .filter_map(|&index| rows.get(index as usize))
        .map(|row| row.name.clone())
        .collect()
}

#[path = "table_view/checkboxes.rs"]
mod checkboxes;
#[path = "table_view/columns.rs"]
mod columns;
#[path = "table_view/horizontal.rs"]
mod horizontal;
#[path = "table_view/rebuild.rs"]
mod rebuild;
#[path = "table_view/scrolling.rs"]
mod scrolling;
#[path = "table_view/sorting.rs"]
mod sorting;
