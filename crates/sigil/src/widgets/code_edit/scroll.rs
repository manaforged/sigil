//! Viewport virtualization: the scrolled row column, the wheel handler, and the
//! caret-follow logic. Owns the math that turns a scroll offset + a real viewport
//! height into the small window of rows [`render`](super::render) actually paints.

use sigil_events::prelude::{ScrollDelta, ScrollEvent};
use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;

use super::render::{GutterCtx, RowCtx, gutter_width, order, rows};
use super::{ROW_BUFFER, Surface, Window};

/// The row column inside the clipping viewport. It holds only the materialized window of
/// rows (see [`render::rows`](super::render::rows)), rebuilt when the first visible row
/// (`start`), the budget, the caret, or the highlights change. `shrink(0)` keeps the rows
/// rigid so flex never compresses them (the recurring overlap bug); the viewport clips
/// whatever overflows.
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
            // The active selection range, ordered (start <= end) and dropped if collapsed
            // onto the caret. Subscribes to the anchor signal so selecting repaints.
            let sel = s
                .selection
                .and_then(|sig| sig.get())
                .map(|anchor| order(anchor, caret))
                .filter(|(a, b)| a != b);
            // The per-line change marks, fetched once per frame (subscribes to the gutter
            // signal so a new diff repaints). Held in a local so its slice can be borrowed
            // across the `rows` call below.
            let marks = s.gutter.map(|g| g.get());
            let gutter = marks.as_deref().map(|marks| GutterCtx {
                marks,
                width: gutter_width(lines.len()),
            });
            // Diagnostic spans, fetched once per frame (subscribes to the signal so a new
            // diagnostics set repaints). Held in a local so its slice outlives `rows`.
            let diagnostics = s.diagnostics.map(|d| d.get()).unwrap_or_default();
            // Window sized to the real viewport height (published by `size_reactive`):
            // exactly the rows the pane can show, plus a small buffer. Subscribes to
            // `viewport`, so a resize rebuilds the window at the new budget.
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
                    selection: sel,
                    gutter,
                    diagnostics: &diagnostics,
                },
            )
        })
}

/// Rows the viewport can show at `row_h`, at least one.
fn visible_rows(viewport_h: f32, row_h: f32) -> usize {
    if row_h <= 0.0 {
        return 1;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rows = (viewport_h / row_h).floor() as usize;
    rows.max(1)
}

/// How many rows to MATERIALIZE: the visible count plus a buffer for a partial bottom row
/// and a little scroll headroom. Only these become nodes (the virtualization budget), so
/// per-frame cost tracks the pane size, not the document length.
fn render_budget(viewport_h: f32, row_h: f32) -> usize {
    visible_rows(viewport_h, row_h) + ROW_BUFFER
}

/// Translate a wheel / trackpad delta into a scroll-offset update, clamped to the
/// content. Sign and feel match the shared [`scroll`](super::super::scroll) widget so the
/// editor scrolls the same direction as every other surface. Only consumes the event when
/// the offset actually moved, so at the top / bottom it bubbles to an outer scroller.
pub(super) fn wheel(
    ev: &mut ScrollEvent,
    scroll_y: Signal<f32>,
    lines: Signal<Vec<String>>,
    viewport: Signal<(f32, f32)>,
    row_h: f32,
) {
    let dy = match ev.delta {
        ScrollDelta::Pixels(p) => p.y,
        ScrollDelta::Lines(l) => l.y * row_h,
    };
    let max = max_scroll(lines.peek().len(), viewport.peek().1, row_h);
    let current = scroll_y.peek();
    let next = (current - dy).clamp(0.0, max);
    if (next - current).abs() > f32::EPSILON {
        scroll_y.set(next);
        ev.stop_propagation();
    }
}

/// The furthest the view can scroll: enough to bring the content's BOTTOM to the pane's
/// bottom, and no further — so a document that fits the pane can't scroll at all (no
/// blank space below). `(content_height - viewport_height).max(0)`, matching the sibling
/// `virtual_list`. Needs the real viewport height, now available via `size_reactive`.
fn max_scroll(line_count: usize, viewport_h: f32, row_h: f32) -> f32 {
    // A real document's line count is far below f32's 2^23 exact-integer range.
    #[allow(clippy::cast_precision_loss)]
    let content = line_count as f32 * row_h;
    (content - viewport_h).max(0.0)
}

/// First visible row for a scroll offset, snapped down to a line and clamped so the
/// window always lands on real content.
pub(super) fn first_row(scroll_y: f32, total: usize, row_h: f32) -> usize {
    if row_h <= 0.0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let row = (scroll_y / row_h).max(0.0).floor() as usize;
    row.min(total.saturating_sub(1))
}

/// Caret-follow: if the caret has moved above the window or past the follow band below
/// it, snap `scroll_y` so the caret's row is shown. A no-op while the caret stays within
/// the band, so ordinary wheel-scrolling is never fought. Subscribes to `viewport` as well
/// as `caret`, so SHRINKING the pane (which narrows the band) also re-pulls the caret into
/// view rather than leaving it stranded below the fold until the next keystroke.
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
    // The follow band is the real visible-row count, so the caret is kept within the
    // actual viewport (not a fixed guess). One row of slack keeps the last line fully on.
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
    // Clamp to the same content-aware bound the wheel uses, so following the caret near
    // the end of a short doc can't scroll past the content into blank space.
    #[allow(clippy::cast_precision_loss)]
    let target_px = (target_row as f32 * row_h).min(max_scroll(total, viewport_h, row_h));
    scroll_y.set(target_px);
}
