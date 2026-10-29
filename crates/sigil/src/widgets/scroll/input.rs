use std::cell::Cell;

use crate::events::selection::drag_active;
use crate::events::{Key, KeyEvent, NodeEventsExt, ScrollDelta, ScrollEvent};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, effect, untrack};

use super::SCROLL_LINE;
use crate::widgets::interaction::{Step, step_keys};

const SCROLL_KEYS: [(Key, Step); 6] = [
    (Key::ArrowDown, Step::Unit(1.0)),
    (Key::ArrowUp, Step::Unit(-1.0)),
    (Key::PageDown, Step::Page(1.0)),
    (Key::PageUp, Step::Page(-1.0)),
    (Key::Home, Step::Start),
    (Key::End, Step::End),
];

fn travel(kind: Step, event: &mut KeyEvent, scroll_y: Signal<f32>, max: f32, page: f32) {
    match kind {
        Step::Unit(sign) => step(event, scroll_y, sign * SCROLL_LINE, max),
        Step::Page(sign) => step(event, scroll_y, sign * page, max),
        Step::Start | Step::End => {
            event.stop_propagation();
            scroll_y.set(if kind == Step::End { max } else { 0.0 });
        }
    }
}

pub(super) fn bind(
    root: NodeBuilder,
    scroll_y: Signal<f32>,
    max: Signal<f32>,
    height: Signal<f32>,
) -> NodeBuilder {
    step_keys(root, &SCROLL_KEYS, move |kind, event| {
        travel(kind, event, scroll_y, max.peek(), height.peek());
    })
    .on_scroll(move |event| {
        let y = WheelAxis {
            offset: scroll_y,
            viewport: height.peek(),
            max: max.peek(),
        };
        wheel(event, Some(y), None);
    })
}

pub(crate) fn contain_scroll(mut root: NodeBuilder) -> NodeBuilder {
    for (key, _) in SCROLL_KEYS {
        root = root.key(key, KeyEvent::stop_propagation);
    }
    root.on_scroll(ScrollEvent::stop_propagation)
}

fn step(event: &mut KeyEvent, scroll_y: Signal<f32>, delta: f32, max: f32) {
    let current = scroll_y.peek();
    let next = (current + delta).clamp(0.0, max);
    if (next - current).abs() > f32::EPSILON {
        scroll_y.set(next);
        event.stop_propagation();
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ScrollAxis {
    pub(crate) offset: Signal<f32>,
    pub(crate) max: Signal<f32>,
    pub(crate) viewport: Signal<f32>,
}

#[derive(Clone, Copy)]
pub(crate) struct WheelAxis {
    pub(crate) offset: Signal<f32>,
    pub(crate) viewport: f32,
    pub(crate) max: f32,
}

impl From<ScrollAxis> for WheelAxis {
    fn from(axis: ScrollAxis) -> Self {
        Self {
            offset: axis.offset,
            viewport: axis.viewport.peek(),
            max: axis.max.peek(),
        }
    }
}

pub(crate) fn wheel(event: &mut ScrollEvent, y: Option<WheelAxis>, x: Option<WheelAxis>) {
    let height = y.or(x).map_or(0.0, |axis| axis.viewport);
    let width = x.map_or(height, |axis| axis.viewport);
    let (dx, dy) = deltas(event, height, width);
    let moved_y = y.is_some_and(|axis| apply(axis.offset, dy, axis.max));
    let moved_x = x.is_some_and(|axis| apply(axis.offset, dx, axis.max));
    if moved_y | moved_x {
        event.stop_propagation();
    }
}

fn deltas(event: &ScrollEvent, height: f32, width: f32) -> (f32, f32) {
    match event.delta {
        ScrollDelta::Pixels(point) => (point.x, point.y),
        ScrollDelta::Lines(lines) => (
            (lines.x * SCROLL_LINE).clamp(-width, width),
            (lines.y * SCROLL_LINE).clamp(-height, height),
        ),
    }
}

fn apply(scroll: Signal<f32>, delta: f32, max: f32) -> bool {
    let current = scroll.peek();
    let next = (current - delta).clamp(0.0, max);
    let moved = (next - current).abs() > f32::EPSILON;
    if moved {
        scroll.set(next);
    }
    moved
}

pub(crate) fn pin_to_bottom(
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    when: impl Fn() -> bool + 'static,
) {
    let previous = Cell::new(max_scroll.peek());
    effect(move || {
        let max = max_scroll.get();
        if drag_active() {
            return;
        }
        let was_pinned = scroll_y.peek() + 1.0 >= previous.replace(max);
        if was_pinned && untrack(&when) {
            scroll_y.set(max);
        }
    });
}
