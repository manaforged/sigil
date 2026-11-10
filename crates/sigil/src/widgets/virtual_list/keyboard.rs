use crate::events::KeyEvent;
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::widgets::Widget;

use crate::widgets::interaction::InteractionState;
use crate::widgets::listbox::{LIST, Move, RowSource};

use super::BuildAxes;
use super::scroll::viewport_px;
use super::window::RowHeights;

#[derive(Clone, Copy)]
pub(super) struct KeyboardRuntime {
    axes: BuildAxes,
    rows: Signal<usize>,
}

impl KeyboardRuntime {
    pub(super) const fn new(axes: BuildAxes, rows: Signal<usize>) -> Self {
        Self { axes, rows }
    }

    fn count(self) -> usize {
        self.rows.peek()
    }

    fn step(self, event: &mut KeyEvent, step: Move) {
        event.stop_propagation();
        let from = self.axes.focused_row.peek();
        let index = LIST.target(&self, from, step).unwrap_or(from);
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
    fn paged(self, from: usize, forward: bool) -> usize {
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
        let target = crate::widgets::scroll::nearest((top, height), current, viewport);
        scroll_y.set(target.clamp(0.0, self.axes.max.peek()));
    }
}

impl RowSource for KeyboardRuntime {
    fn len(&self) -> usize {
        self.count()
    }

    fn page(&self, from: usize, forward: bool) -> Option<usize> {
        Some(self.paged(from, forward))
    }
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: KeyboardRuntime) -> NodeBuilder {
    let interaction = InteractionState::new(true);
    let mut node = node.focusable(true).focus_ring(interaction.focus_ring());
    for (key, step) in LIST.bindings() {
        node = node.key(key, move |event| runtime.step(event, step));
    }
    interaction.observe_style(node)
}
