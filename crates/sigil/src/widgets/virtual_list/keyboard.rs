use crate::events::prelude::{Key, KeyEvent, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::style::NodeStyleExt;
use crate::widgets::interaction::InteractionState;

use super::BuildAxes;
use super::scroll::viewport_px;
use super::window::RowHeights;

#[derive(Clone, Copy)]
pub(super) struct KeyboardRuntime {
    axes: BuildAxes,
    total_rows: usize,
    total_rows_live: Option<Signal<usize>>,
}

impl KeyboardRuntime {
    pub(super) const fn new(
        axes: BuildAxes,
        total_rows: usize,
        total_rows_live: Option<Signal<usize>>,
    ) -> Self {
        Self {
            axes,
            total_rows,
            total_rows_live,
        }
    }

    fn count(self) -> usize {
        self.total_rows_live.map_or(self.total_rows, Signal::peek)
    }

    fn step(self, event: &mut KeyEvent, next: impl FnOnce(Self, usize) -> usize) {
        event.stop_propagation();
        let index = next(self, self.axes.focused_row.peek());
        let focused = index.min(self.count().saturating_sub(1));
        self.axes.focused_row.set(focused);
        self.scroll_into_view(focused);
    }

    fn viewport(self) -> f32 {
        viewport_px(self.axes.viewport.peek(), self.axes.viewport_height)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    fn page(self, from: usize, forward: bool) -> usize {
        let viewport = self.viewport();
        if let Some(tops) = self.axes.tops {
            let tops = tops.peek();
            let heights = RowHeights::Tops(&tops);
            let top = heights.top(from);
            return if forward {
                heights.row_at(top + viewport).max(from.saturating_add(1))
            } else {
                heights.row_at(top - viewport).min(from.saturating_sub(1))
            };
        }
        let rows = (viewport / self.axes.row_height).ceil().max(1.0) as usize;
        if forward {
            from.saturating_add(rows)
        } else {
            from.saturating_sub(rows)
        }
    }

    fn span(self, index: usize) -> (f32, f32) {
        self.axes.tops.map_or_else(
            || RowHeights::fixed(index.saturating_add(1), self.axes.row_height).span(index),
            |tops| RowHeights::Tops(&tops.peek()).span(index),
        )
    }

    fn scroll_into_view(self, focused: usize) {
        let scroll_y = self.axes.scroll_y;
        if self.count() == 0 {
            scroll_y.set(0.0);
            return;
        }
        let (top, height) = self.span(focused);
        let viewport = self.viewport();
        let current = scroll_y.peek();
        let target = if top < current {
            top
        } else if top + height > current + viewport {
            top + height - viewport
        } else {
            current
        };
        scroll_y.set(target.clamp(0.0, self.axes.max.peek()));
    }
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: KeyboardRuntime) -> NodeBuilder {
    let interaction = InteractionState::new(true);
    let node = node
        .focusable(true)
        .focus_ring(interaction.focus_ring())
        .key(Key::ArrowDown, move |event| {
            runtime.step(event, |_, row| row.saturating_add(1));
        })
        .key(Key::ArrowUp, move |event| {
            runtime.step(event, |_, row| row.saturating_sub(1));
        })
        .key(Key::PageDown, move |event| {
            runtime.step(event, |list, row| list.page(row, true));
        })
        .key(Key::PageUp, move |event| {
            runtime.step(event, |list, row| list.page(row, false));
        })
        .key(Key::Home, move |event| runtime.step(event, |_, _| 0))
        .key(Key::End, move |event| {
            runtime.step(event, |list, _| list.count().saturating_sub(1));
        });
    interaction.observe_style(node)
}
