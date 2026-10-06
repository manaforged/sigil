use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::events::NodeEventsExt;
use crate::events::selection::drag_active;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, derived, effect, untrack};

use super::measure::Tops;
use super::window::RowHeights;
use super::{BuildAxes, InitialScroll, RowRender, VirtualListBuilder};
use crate::widgets::scroll::{WheelAxis, pin_to_bottom, wheel};

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    #[must_use]
    pub const fn content_height_bind(mut self, content_height: Signal<f32>) -> Self {
        self.content_height_bind = Some(content_height);
        self
    }

    #[must_use]
    pub const fn max_scroll_bind(mut self, max_scroll: Signal<f32>) -> Self {
        self.max_scroll_bind = Some(max_scroll);
        self
    }
}

pub(super) fn content_px(tops: Option<Tops>, row_height: f32, rows: usize) -> f32 {
    tops.map_or_else(
        || RowHeights::fixed(rows, row_height).total(),
        |tops| RowHeights::Tops(&tops.get()).total(),
    )
}

pub(super) fn extent(
    viewport: Signal<(f32, f32)>,
    viewport_height: f32,
    content: impl Fn() -> f32 + 'static,
) -> (Signal<f32>, Signal<f32>) {
    let content = derived(content);
    let max =
        derived(move || (content.get() - viewport_px(viewport.get(), viewport_height)).max(0.0));
    (content, max)
}

pub(super) fn bind_extent(axes: BuildAxes, content: Option<Signal<f32>>, max: Option<Signal<f32>>) {
    if let Some(out) = content {
        effect(move || out.set(axes.content.get()));
    }
    if let Some(out) = max {
        effect(move || out.set(axes.max.get()));
    }
}

fn row_span(axes: BuildAxes, index: usize) -> (f32, f32) {
    axes.tops.map_or_else(
        || RowHeights::fixed(index.saturating_add(1), axes.row_height).span(index),
        |tops| RowHeights::Tops(&tops.get()).span(index),
    )
}

pub(super) fn viewport_px(measured: (f32, f32), fallback: f32) -> f32 {
    if measured.1 > 0.0 {
        measured.1
    } else {
        fallback
    }
}

fn follow_target(axes: BuildAxes, (top, height): (f32, f32), current: f32, viewport: f32) -> f32 {
    let bottom = top + height;
    if axes.tops.is_some() {
        if bottom > current + viewport {
            bottom - viewport
        } else if top < current {
            top
        } else {
            current
        }
    } else if top < current {
        top
    } else if bottom > current + viewport {
        ((bottom - viewport) / height).ceil() * height
    } else {
        current
    }
}

pub(super) fn follow(target: Signal<usize>, axes: BuildAxes, rows: impl Fn() -> usize + 'static) {
    let rows = Rc::new(rows);
    let tail_rows = Rc::clone(&rows);
    pin_to_bottom(axes.scroll_y, axes.max, move || {
        target.peek().saturating_add(1) >= tail_rows()
    });
    let last_rows = Cell::new(rows());
    effect(move || {
        let wanted = target.get();
        untrack(|| {
            if drag_active() {
                return;
            }
            let total = rows();
            let current = axes.scroll_y.peek();
            let max = axes.max.peek();
            if last_rows.replace(total) != total && current + 1.0 < max {
                return;
            }
            let viewport = viewport_px(axes.viewport.peek(), axes.viewport_height);
            let span = row_span(axes, wanted.min(total.saturating_sub(1)));
            let next = follow_target(axes, span, current, viewport).clamp(0.0, max);
            if (next - current).abs() > f32::EPSILON {
                axes.scroll_y.set(next);
            }
        });
    });
}

pub(super) fn anchor(axes: BuildAxes) {
    let Some(tops) = axes.tops else {
        return;
    };
    let seen = RefCell::new(tops.peek());
    let seen_keys = RefCell::new(axes.keys.map(|keys| keys.peek()));
    effect(move || {
        let next = tops.get();
        untrack(|| {
            let prev = seen.replace(Rc::clone(&next));
            let next_keys = axes.keys.map(|keys| keys.peek());
            let prev_keys = seen_keys.replace(next_keys.clone());
            let current = axes.scroll_y.peek();
            let viewport = viewport_px(axes.viewport.peek(), axes.viewport_height);
            let (before, after) = (RowHeights::Tops(&prev), RowHeights::Tops(&next));
            if current + 1.0 >= before.total() - viewport {
                return;
            }
            let row = before.row_at(current);
            let moved_to = same_item(prev_keys.as_deref(), next_keys.as_deref(), row);
            let shift = after.top(moved_to) - before.top(row);
            if shift.abs() > f32::EPSILON {
                let max = (after.total() - viewport).max(0.0);
                axes.scroll_y.set((current + shift).clamp(0.0, max));
            }
        });
    });
}

fn same_item(before: Option<&Vec<u64>>, after: Option<&Vec<u64>>, row: usize) -> usize {
    let (Some(before), Some(after)) = (before, after) else {
        return row;
    };
    let Some(key) = before.get(row) else {
        return row;
    };
    if after.get(row) == Some(key) {
        return row;
    }
    after
        .iter()
        .position(|candidate| candidate == key)
        .unwrap_or(row)
}

pub(super) fn apply_bound_scroll(
    axes: BuildAxes,
    initial: Option<InitialScroll>,
    pin_bottom: bool,
) {
    if let Some(initial) = initial {
        let max = axes.max.peek();
        let target = match initial {
            InitialScroll::End => max,
            InitialScroll::Row(index) => row_span(axes, index).0.clamp(0.0, max),
        };
        axes.scroll_y.set(target);
    }
    if pin_bottom {
        pin_to_bottom(axes.scroll_y, axes.max, || true);
    }
}

pub(super) fn attach_wheel(root: NodeBuilder, axes: BuildAxes) -> NodeBuilder {
    let scroll_y = axes.scroll_y;
    let scroll_x = axes.scroll_x;
    root.on_scroll(move |ev| {
        let y = WheelAxis {
            offset: scroll_y,
            viewport: viewport_px(axes.viewport.peek(), axes.viewport_height),
            max: axes.max.peek(),
        };
        wheel(ev, Some(y), scroll_x.map(WheelAxis::from));
    })
}
