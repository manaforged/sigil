use crate::events::prelude::NodeEventsExt;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, derived_eq};

use super::window::{row_top, total_height};
use super::{BuildAxes, InitialScroll};
use crate::widgets::scroll::{pin_to_bottom as follow_bottom, wheel, wheel2};

fn max_scroll(rows: usize, row_height: f32, viewport_height: f32) -> f32 {
    (total_height(rows, row_height) - viewport_height).max(0.0)
}

pub(super) fn seed_bound_scroll(
    scroll_y: Signal<f32>,
    initial: Option<InitialScroll>,
    rows: usize,
    row_height: f32,
    viewport_height: f32,
) {
    let Some(initial) = initial else {
        return;
    };
    let max = max_scroll(rows, row_height, viewport_height);
    let target = match initial {
        InitialScroll::End => max,
        InitialScroll::Row(index) => row_top(index, row_height).clamp(0.0, max),
    };
    scroll_y.set(target);
}

pub(super) fn apply_bound_scroll(
    axes: BuildAxes,
    initial: Option<InitialScroll>,
    pin_bottom: bool,
    initial_rows: usize,
    live_rows: Signal<usize>,
) {
    seed_bound_scroll(
        axes.scroll_y,
        initial,
        initial_rows,
        axes.row_height,
        axes.viewport_height,
    );
    if pin_bottom {
        pin_to_bottom(
            axes.scroll_y,
            live_rows,
            axes.row_height,
            axes.viewport_height,
        );
    }
}

fn pin_to_bottom(
    scroll_y: Signal<f32>,
    live_rows: Signal<usize>,
    row_height: f32,
    viewport_height: f32,
) {
    let max = derived_eq(move || max_scroll(live_rows.get(), row_height, viewport_height));
    follow_bottom(scroll_y, max);
}

pub(super) fn attach_wheel(
    root: NodeBuilder,
    axes: BuildAxes,
    max_scroll: impl Fn() -> f32 + 'static,
) -> NodeBuilder {
    let scroll_y = axes.scroll_y;
    let viewport_height = axes.viewport_height;
    let scroll_x = axes.scroll_x;
    root.on_scroll(move |ev| match scroll_x {
        Some(x) => wheel2(ev, scroll_y, viewport_height, max_scroll(), x),
        None => wheel(ev, scroll_y, viewport_height, max_scroll()),
    })
}
