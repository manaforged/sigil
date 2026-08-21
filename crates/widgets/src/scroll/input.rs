use std::cell::Cell;

use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt, ScrollDelta, ScrollEvent};
use sigil_kernel::NodeBuilder;
use sigil_reactive::{Signal, effect};

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
    let dy = match event.delta {
        ScrollDelta::Pixels(point) => point.y,
        ScrollDelta::Lines(lines) => (lines.y * SCROLL_LINE).clamp(-height, height),
    };
    let current = scroll_y.peek();
    let next = (current - dy).clamp(0.0, max);
    if (next - current).abs() > f32::EPSILON {
        scroll_y.set(next);
        event.stop_propagation();
    }
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
