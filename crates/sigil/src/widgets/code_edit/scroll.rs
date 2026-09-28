use crate::events::prelude::ScrollEvent;
use crate::kernel::{NodeBuilder, container};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;

use super::render::{GutterCtx, RowCtx, gutter_width, order, rows};
use super::{ROW_BUFFER, Surface, Window};

pub(super) fn scroll_body(s: Surface, row_h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .align_items(AlignItems::Start)
                .shrink(0.0),
        )
        .children_reactive(move || {
            let lines = s.lines.get();
            let hl = s.highlights.map(|h| h.get()).unwrap_or_default();
            let caret = s.caret.get();
            let sel = s
                .selection
                .and_then(|sig| sig.get())
                .map(|anchor| order(anchor, caret))
                .filter(|(a, b)| a != b);
            let marks = s.gutter.map(|g| g.get());
            let gutter = marks.as_deref().map(|marks| GutterCtx {
                marks,
                width: gutter_width(lines.len()),
            });
            let diagnostics = s.diagnostics.map(|d| d.get()).unwrap_or_default();
            let win = Window {
                start: s.start.get(),
                budget: render_budget(s.viewport.get().1, row_h),
                row_h,
            };
            rows(
                &lines,
                &hl,
                win,
                RowCtx {
                    caret,
                    caret_shown: s.caret_shown.get(),
                    selection: sel,
                    gutter,
                    diagnostics: &diagnostics,
                },
            )
        })
}

fn visible_rows(viewport_h: f32, row_h: f32) -> usize {
    if row_h <= 0.0 {
        return 1;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let rows = (viewport_h / row_h).floor() as usize;
    rows.max(1)
}

fn render_budget(viewport_h: f32, row_h: f32) -> usize {
    visible_rows(viewport_h, row_h) + ROW_BUFFER
}

pub(super) fn wheel(
    ev: &mut ScrollEvent,
    scroll_y: Signal<f32>,
    lines: Signal<Vec<String>>,
    viewport: Signal<(f32, f32)>,
    row_h: f32,
) {
    let height = viewport.peek().1;
    let max = max_scroll(lines.peek().len(), height, row_h);
    let y = crate::widgets::scroll::WheelAxis {
        offset: scroll_y,
        viewport: height,
        max,
    };
    crate::widgets::scroll::wheel(ev, Some(y), None);
}

fn max_scroll(line_count: usize, viewport_h: f32, row_h: f32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let content = line_count as f32 * row_h;
    (content - viewport_h).max(0.0)
}

pub(super) fn first_row(scroll_y: f32, total: usize, row_h: f32) -> usize {
    if row_h <= 0.0 {
        return 0;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let row = (scroll_y / row_h).max(0.0).floor() as usize;
    row.min(total.saturating_sub(1))
}

pub(super) fn follow_caret(
    scroll_y: Signal<f32>,
    caret: Signal<(usize, usize)>,
    lines: Signal<Vec<String>>,
    viewport: Signal<(f32, f32)>,
    row_h: f32,
) {
    let (caret_line, _) = caret.get();
    let total = lines.peek().len();
    if total == 0 {
        return;
    }
    let viewport_h = viewport.get().1;
    let band = visible_rows(viewport_h, row_h).saturating_sub(1).max(1);
    let top = first_row(scroll_y.peek(), total, row_h);
    let target_row = if caret_line < top {
        caret_line
    } else if caret_line >= top + band {
        caret_line + 1 - band
    } else {
        return;
    };
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let target_px = (target_row as f32 * row_h).min(max_scroll(total, viewport_h, row_h));
    scroll_y.set(target_px);
}
