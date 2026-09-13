use std::cell::Cell;

use crate::events::prelude::{Key, KeyEvent, NodeEventsExt, ScrollDelta, ScrollEvent};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, effect};

use super::SCROLL_LINE;

pub(super) fn bind(
    root: NodeBuilder,
    scroll_y: Signal<f32>,
    max: Signal<f32>,
    height: Signal<f32>,
) -> NodeBuilder {
    root.key(Key::ArrowDown, move |event| {
        step(event, scroll_y, SCROLL_LINE, max.peek());
    })
    .key(Key::ArrowUp, move |event| {
        step(event, scroll_y, -SCROLL_LINE, max.peek());
    })
    .key(Key::PageDown, move |event| {
        step(event, scroll_y, height.peek(), max.peek());
    })
    .key(Key::PageUp, move |event| {
        step(event, scroll_y, -height.peek(), max.peek());
    })
    .key(Key::Home, move |event| jump(event, scroll_y, 0.0))
    .key(Key::End, move |event| jump(event, scroll_y, max.peek()))
    .on_scroll(move |event| wheel(event, scroll_y, height.peek(), max.peek()))
}

fn step(event: &mut KeyEvent, scroll_y: Signal<f32>, delta: f32, max: f32) {
    let current = scroll_y.peek();
    let next = (current + delta).clamp(0.0, max);
    if (next - current).abs() > f32::EPSILON {
        scroll_y.set(next);
        event.stop_propagation();
    }
}

fn jump(event: &mut KeyEvent, scroll_y: Signal<f32>, target: f32) {
    event.stop_propagation();
    scroll_y.set(target);
}

pub(crate) fn wheel(event: &mut ScrollEvent, scroll_y: Signal<f32>, height: f32, max: f32) {
    let (_, dy) = deltas(event, height, height);
    if apply(scroll_y, dy, max) {
        event.stop_propagation();
    }
}

pub(crate) type WheelX = (Signal<f32>, Signal<f32>, Signal<f32>);

pub(crate) fn wheel2(
    event: &mut ScrollEvent,
    scroll_y: Signal<f32>,
    height: f32,
    max_y: f32,
    x: WheelX,
) {
    let (dx, dy) = deltas(event, height, x.2.peek());
    let moved = apply(scroll_y, dy, max_y) | apply(x.0, dx, x.1.peek());
    if moved {
        event.stop_propagation();
    }
}

pub(crate) fn wheel_x(event: &mut ScrollEvent, scroll_x: Signal<f32>, width: f32, max: f32) {
    let (dx, _) = deltas(event, width, width);
    if apply(scroll_x, dx, max) {
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
    if (next - current).abs() > f32::EPSILON {
        scroll.set(next);
        return true;
    }
    false
}

pub(crate) fn pin_to_bottom(scroll_y: Signal<f32>, max_scroll: Signal<f32>) {
    let previous = Cell::new(max_scroll.peek());
    effect(move || {
        let max = max_scroll.get();
        let was_pinned = scroll_y.peek() + 1.0 >= previous.replace(max);
        if was_pinned {
            scroll_y.set(max);
        }
    });
}
