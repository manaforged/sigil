use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::events::prelude::NodeEventsExt;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, derived_eq, effect, untrack};

use super::measure::Tops;
use super::window::{prefix_row_top, row_top, total_height, total_height_variable, y_to_row};
use super::{BuildAxes, InitialScroll, VirtualListBuilder};
use crate::widgets::scroll::{pin_to_bottom as follow_bottom, wheel, wheel2};

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
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
        || total_height(rows, row_height),
        |tops| total_height_variable(&tops.get()),
    )
}

pub(super) fn extent(
    viewport: Signal<(f32, f32)>,
    viewport_height: f32,
    content: impl Fn() -> f32 + 'static,
) -> (Signal<f32>, Signal<f32>) {
    let content = derived_eq(content);
    let max =
        derived_eq(move || (content.get() - viewport_px(viewport.get(), viewport_height)).max(0.0));
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
        || (row_top(index, axes.row_height), axes.row_height),
        |tops| {
            let tops = tops.get();
            let top = prefix_row_top(&tops, index);
            (top, prefix_row_top(&tops, index + 1) - top)
        },
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
    follow_bottom(axes.scroll_y, axes.max);
    let last_rows = Cell::new(rows());
    effect(move || {
        let wanted = target.get();
        untrack(|| {
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
    effect(move || {
        let next = tops.get();
        untrack(|| {
            let prev = seen.replace(Rc::clone(&next));
            let current = axes.scroll_y.peek();
            let viewport = viewport_px(axes.viewport.peek(), axes.viewport_height);
            if current + 1.0 >= total_height_variable(&prev) - viewport {
                return;
            }
            let row = y_to_row(&prev, current);
            let shift = prefix_row_top(&next, row) - prefix_row_top(&prev, row);
            if shift.abs() > f32::EPSILON {
                let max = (total_height_variable(&next) - viewport).max(0.0);
                axes.scroll_y.set((current + shift).clamp(0.0, max));
            }
        });
    });
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
        follow_bottom(axes.scroll_y, axes.max);
    }
}

pub(super) fn attach_wheel(root: NodeBuilder, axes: BuildAxes) -> NodeBuilder {
    let scroll_y = axes.scroll_y;
    let scroll_x = axes.scroll_x;
    root.on_scroll(move |ev| {
        let viewport_height = viewport_px(axes.viewport.peek(), axes.viewport_height);
        let max = axes.max.peek();
        match scroll_x {
            Some(x) => wheel2(ev, scroll_y, viewport_height, max, x),
            None => wheel(ev, scroll_y, viewport_height, max),
        }
    })
}
