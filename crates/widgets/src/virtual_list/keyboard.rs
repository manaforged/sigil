use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt};
use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

use super::window::{positive_px, prefix_row_top, row_top, total_height, total_height_variable};
use super::{DEFAULT_ROW_HEIGHT, DEFAULT_VIEWPORT_HEIGHT};

#[derive(Clone)]
pub(super) struct KeyboardRuntime {
    total_rows: usize,
    total_rows_live: Option<Signal<usize>>,
    row_height: f32,
    viewport_height: f32,
    scroll_y: Signal<f32>,
    focused_row: Signal<usize>,
    row_tops: Option<Vec<f32>>,
}

pub(super) struct KeyboardRuntimeInit {
    pub(super) total_rows: usize,
    pub(super) total_rows_live: Option<Signal<usize>>,
    pub(super) row_height: f32,
    pub(super) viewport_height: f32,
    pub(super) scroll_y: Signal<f32>,
    pub(super) focused_row: Signal<usize>,
    pub(super) row_tops: Option<Vec<f32>>,
}

impl KeyboardRuntime {
    pub(super) fn new(init: KeyboardRuntimeInit) -> Self {
        Self {
            total_rows: init.total_rows,
            total_rows_live: init.total_rows_live,
            row_height: init.row_height,
            viewport_height: init.viewport_height,
            scroll_y: init.scroll_y,
            focused_row: init.focused_row,
            row_tops: init.row_tops,
        }
    }

    fn count(&self) -> usize {
        self.total_rows_live.map_or(self.total_rows, Signal::get)
    }

    fn move_forward(self, event: &mut KeyEvent, rows: usize) {
        event.stop_propagation();
        let next = self.focused_row.peek().saturating_add(rows);
        self.set_focus(next);
    }

    fn move_back(self, event: &mut KeyEvent, rows: usize) {
        event.stop_propagation();
        let next = self.focused_row.peek().saturating_sub(rows);
        self.set_focus(next);
    }

    fn jump(self, event: &mut KeyEvent, target: usize) {
        event.stop_propagation();
        self.set_focus(target);
    }

    fn set_focus(&self, index: usize) {
        let max_row = self.count().saturating_sub(1);
        let focused = index.min(max_row);
        self.focused_row.set(focused);
        self.scroll_focused_row_into_view(focused);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    fn page_step(&self) -> usize {
        let avg = self.row_tops.as_ref().map_or(self.row_height, |row_tops| {
            self.average_variable_row_height(row_tops)
        });
        let avg = positive_px(avg, self.viewport_height / 16.0);
        (self.viewport_height / avg).ceil().max(1.0) as usize
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    fn average_variable_row_height(&self, row_tops: &[f32]) -> f32 {
        let count = self.count();
        if count == 0 {
            DEFAULT_ROW_HEIGHT
        } else {
            total_height_variable(row_tops) / count as f32
        }
    }

    fn scroll_focused_row_into_view(&self, focused: usize) {
        if self.count() == 0 {
            self.scroll_y.set(0.0);
            return;
        }
        let top = self.row_top(focused);
        let bottom = top + self.row_height_at(focused);
        let viewport_height = positive_px(self.viewport_height, DEFAULT_VIEWPORT_HEIGHT);
        let current_top = self.scroll_y.peek();
        let current_bottom = current_top + viewport_height;
        let target = if top < current_top {
            top
        } else if bottom > current_bottom {
            bottom - viewport_height
        } else {
            current_top
        };
        self.scroll_y.set(target.clamp(0.0, self.max_scroll()));
    }

    fn row_top(&self, index: usize) -> f32 {
        self.row_tops.as_ref().map_or_else(
            || row_top(index, self.row_height),
            |row_tops| prefix_row_top(row_tops, index),
        )
    }

    fn row_height_at(&self, index: usize) -> f32 {
        self.row_tops.as_ref().map_or(self.row_height, |row_tops| {
            let next = prefix_row_top(row_tops, index.saturating_add(1));
            next - prefix_row_top(row_tops, index)
        })
    }

    fn max_scroll(&self) -> f32 {
        let content_height = self.row_tops.as_ref().map_or_else(
            || total_height(self.count(), self.row_height),
            |row_tops| total_height_variable(row_tops),
        );
        (content_height - self.viewport_height).max(0.0)
    }
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: KeyboardRuntime) -> NodeBuilder {
    let arrow_down = runtime.clone();
    let arrow_up = runtime.clone();
    let page_down = runtime.clone();
    let page_up = runtime.clone();
    let home = runtime.clone();
    let end = runtime;

    node.focusable(true)
        .key(Key::ArrowDown, move |event| {
            arrow_down.clone().move_forward(event, 1);
        })
        .key(Key::ArrowUp, move |event| {
            arrow_up.clone().move_back(event, 1);
        })
        .key(Key::PageDown, move |event| {
            let rows = page_down.page_step();
            page_down.clone().move_forward(event, rows);
        })
        .key(Key::PageUp, move |event| {
            let rows = page_up.page_step();
            page_up.clone().move_back(event, rows);
        })
        .key(Key::Home, move |event| {
            home.clone().jump(event, 0);
        })
        .key(Key::End, move |event| {
            let last = end.count().saturating_sub(1);
            end.clone().jump(event, last);
        })
}
