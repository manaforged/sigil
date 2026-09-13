use super::{DEFAULT_ROW_HEIGHT, DEFAULT_VIEWPORT_HEIGHT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct VisibleWindow {
    pub(super) first: usize,
    pub(super) end: usize,
}

impl VisibleWindow {
    pub(super) fn len(self) -> usize {
        self.end.saturating_sub(self.first)
    }
}

pub(super) fn positive_px(px: f32, fallback: f32) -> f32 {
    if px.is_finite() && px > 0.0 {
        px
    } else {
        fallback
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub(super) fn page_rows(viewport_height: f32, row_height: f32) -> usize {
    let row_height = positive_px(row_height, DEFAULT_ROW_HEIGHT);
    let viewport_height = positive_px(viewport_height, DEFAULT_VIEWPORT_HEIGHT);
    ((viewport_height / row_height).ceil() as usize).max(1)
}

fn pad_rows(viewport_height: f32, row_height: f32, overscan: usize) -> usize {
    page_rows(viewport_height, row_height).saturating_mul(overscan)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub(super) fn strip_height(viewport_height: f32, row_height: f32, overscan: usize) -> f32 {
    let pad = pad_rows(viewport_height, row_height, overscan) as f32;
    (pad * 2.0).mul_add(row_height, viewport_height)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub(super) fn visible_window(
    scroll_y: f32,
    total: usize,
    row_height: f32,
    viewport_height: f32,
    overscan: usize,
) -> VisibleWindow {
    if total == 0 {
        return VisibleWindow { first: 0, end: 0 };
    }
    let row_height = positive_px(row_height, DEFAULT_ROW_HEIGHT);
    let viewport_height = positive_px(viewport_height, DEFAULT_VIEWPORT_HEIGHT);
    let pad = pad_rows(viewport_height, row_height, overscan);
    let first_f = (scroll_y / row_height).floor().max(0.0);
    let end_f = ((scroll_y + viewport_height) / row_height).ceil();
    let vis_first = (first_f as usize).min(total);
    let vis_end = (end_f as usize).max(vis_first).min(total);
    VisibleWindow {
        first: vis_first.saturating_sub(pad),
        end: vis_end.saturating_add(pad).min(total),
    }
}

pub(super) fn visible_window_variable(
    scroll_y: f32,
    row_tops: &[f32],
    viewport_height: f32,
    overscan: usize,
) -> VisibleWindow {
    let total = row_tops.len().saturating_sub(1);
    if total == 0 {
        return VisibleWindow { first: 0, end: 0 };
    }

    let viewport_height = positive_px(viewport_height, DEFAULT_VIEWPORT_HEIGHT);
    let content_height = total_height_variable(row_tops);
    let scroll_y = finite_scroll_y(scroll_y);
    let (vis_first, vis_end) = if scroll_y >= content_height {
        (total, total)
    } else {
        let viewport_bottom = (scroll_y + viewport_height).min(content_height);
        (
            y_to_row(row_tops, scroll_y).min(total),
            first_row_at_or_after_y(row_tops, viewport_bottom).min(total),
        )
    };
    let first = pad_start_variable(row_tops, vis_first, viewport_height, overscan);
    let end = pad_end_variable(row_tops, vis_end, viewport_height, overscan).min(total);
    VisibleWindow {
        first,
        end: end.max(first),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn pad_start_variable(
    row_tops: &[f32],
    first: usize,
    viewport_height: f32,
    overscan: usize,
) -> usize {
    let top = prefix_row_top(row_tops, first);
    let pad_px = viewport_height * overscan as f32;
    y_to_row(row_tops, (top - pad_px).max(0.0))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn pad_end_variable(row_tops: &[f32], end: usize, viewport_height: f32, overscan: usize) -> usize {
    let top = prefix_row_top(row_tops, end);
    let pad_px = viewport_height * overscan as f32;
    first_row_at_or_after_y(row_tops, top + pad_px)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub(super) fn total_height(rows: usize, row_height: f32) -> f32 {
    rows as f32 * row_height
}

pub(super) fn total_height_variable(row_tops: &[f32]) -> f32 {
    row_tops.last().copied().unwrap_or(0.0)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub(super) fn row_top(index: usize, row_height: f32) -> f32 {
    index as f32 * row_height
}

pub(super) fn prefix_row_top(row_tops: &[f32], index: usize) -> f32 {
    row_tops
        .get(index)
        .copied()
        .unwrap_or_else(|| total_height_variable(row_tops))
}

pub(super) fn y_to_row(row_tops: &[f32], y: f32) -> usize {
    let total = row_tops.len().saturating_sub(1);
    if total == 0 {
        return 0;
    }

    let y = finite_scroll_y(y);
    let partition = row_tops.partition_point(|top| *top <= y);
    partition.saturating_sub(1).min(total.saturating_sub(1))
}

fn first_row_at_or_after_y(row_tops: &[f32], y: f32) -> usize {
    let total = row_tops.len().saturating_sub(1);
    let y = finite_scroll_y(y);
    row_tops.partition_point(|top| *top < y).min(total)
}

fn finite_scroll_y(scroll_y: f32) -> f32 {
    if scroll_y.is_finite() {
        scroll_y.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
