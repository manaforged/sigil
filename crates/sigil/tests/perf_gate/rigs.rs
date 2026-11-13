use sigil::Host;
use sigil::events::{Event, PhysicalKey};
use sigil::geometry::Point;
use sigil::kernel::container;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::reactive::signal;
use sigil::widgets::{button, chart, text_field, virtual_list};

use crate::budgets::Scenario;

pub(crate) const VIEW: (f32, f32) = (1280.0, 800.0);
const ROWS_10K: usize = 10_000;
const BUTTONS: usize = 40;

type EventAt = fn(u32) -> Event;
type TreeFn = fn() -> Box<dyn IntoNode>;

pub(crate) struct Rig {
    pub tree: TreeFn,
    pub setup: Vec<Event>,
    pub events: Option<EventAt>,
}

fn buttons() -> Box<dyn IntoNode> {
    sigil::kernel::node(buttons_tree())
}

fn buttons_tree() -> impl IntoNode {
    let mut col = container().layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1));
    for index in 0..BUTTONS {
        col = col.child(button(format!("action {index}")));
    }
    col
}

fn rows() -> Box<dyn IntoNode> {
    sigil::kernel::node(rows_tree())
}

fn rows_tree() -> impl IntoNode {
    let y = signal(0.0_f32);
    let data: Vec<String> = (0..ROWS_10K).map(|i| format!("row {i}")).collect();
    virtual_list(data, |_, row: &String| container().child(text(row.clone())))
        .row_height(24.0)
        .viewport_height(VIEW.1)
        .viewport_width(VIEW.0)
        .scroll_y(y)
}

fn editor() -> Box<dyn IntoNode> {
    sigil::kernel::node(editor_tree())
}

fn editor_tree() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
        .child(text_field(signal(String::from("hello world\nsecond line"))).multiline())
}

fn plot() -> Box<dyn IntoNode> {
    sigil::kernel::node(plot_tree())
}

fn plot_tree() -> impl IntoNode {
    let values: Vec<f32> = (0..200_u16).map(|i| f32::from(i % 37)).collect();
    container()
        .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
        .child(chart(values).line())
}

const fn hover(frame: u32) -> Event {
    let y = if frame.is_multiple_of(2) { 10.0 } else { 60.0 };
    Event::pointer_move(Point::new(20.0, y))
}

const fn scroll(frame: u32) -> Event {
    let dy = if frame % 50 < 25 { -3.0 } else { 3.0 };
    Event::wheel(Point::new(100.0, 100.0), dy)
}

fn typing(frame: u32) -> Event {
    let ch = if frame.is_multiple_of(2) { "a" } else { "b" };
    Event::typed(ch).physical(PhysicalKey::KeyA)
}

pub(crate) fn rig(scenario: Scenario) -> Rig {
    let (tree, events): (TreeFn, Option<EventAt>) = match scenario {
        Scenario::Idle | Scenario::Terminal => (buttons, None),
        Scenario::Hover => (buttons, Some(hover)),
        Scenario::Scroll => (rows, Some(scroll)),
        Scenario::Typing => (editor, Some(typing)),
        Scenario::Chart => (plot, None),
    };
    let setup = match scenario {
        Scenario::Typing => vec![
            Event::pointer_down(Point::new(20.0, 10.0)),
            Event::pointer_up(Point::new(20.0, 10.0)),
        ],
        Scenario::Scroll => vec![Event::pointer_move(Point::new(100.0, 100.0))],
        _ => Vec::new(),
    };
    Rig {
        tree,
        setup,
        events,
    }
}

pub(crate) fn mount(host: &mut Host, rig: &Rig) {
    host.mount(rig.tree);
    for event in rig.setup.iter().cloned() {
        crate::drive::window_frame(host, Some(event), false);
    }
}
