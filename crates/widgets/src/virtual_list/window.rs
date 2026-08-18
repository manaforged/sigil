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

/// Rows in one viewport. Overscan on each side equals this count.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(super) fn page_rows(viewport_height: f32, row_height: f32) -> usize {
    let row_height = positive_px(row_height, DEFAULT_ROW_HEIGHT);
    let viewport_height = positive_px(viewport_height, DEFAULT_VIEWPORT_HEIGHT);
    ((viewport_height / row_height).ceil() as usize).max(1)
}

/// Host height that fits the viewport plus one page of overscan each side.
#[allow(clippy::cast_precision_loss)]
pub(super) fn strip_height(viewport_height: f32, row_height: f32) -> f32 {
    let pad = page_rows(viewport_height, row_height) as f32;
    (pad * 2.0).mul_add(row_height, viewport_height)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(super) fn visible_window(
    scroll_y: f32,
    total: usize,
    row_height: f32,
    viewport_height: f32,
) -> VisibleWindow {
    if total == 0 {
        return VisibleWindow { first: 0, end: 0 };
    }
    let row_height = positive_px(row_height, DEFAULT_ROW_HEIGHT);
    let viewport_height = positive_px(viewport_height, DEFAULT_VIEWPORT_HEIGHT);
    let pad = page_rows(viewport_height, row_height);
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
    let first = pad_start_variable(row_tops, vis_first, viewport_height);
    let end = pad_end_variable(row_tops, vis_end, viewport_height).min(total);
    VisibleWindow {
        first,
        end: end.max(first),
    }
}

fn pad_start_variable(row_tops: &[f32], first: usize, viewport_height: f32) -> usize {
    let top = prefix_row_top(row_tops, first);
    y_to_row(row_tops, (top - viewport_height).max(0.0))
}

fn pad_end_variable(row_tops: &[f32], end: usize, viewport_height: f32) -> usize {
    let top = prefix_row_top(row_tops, end);
    first_row_at_or_after_y(row_tops, top + viewport_height)
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn total_height(rows: usize, row_height: f32) -> f32 {
    rows as f32 * row_height
}

pub(super) fn total_height_variable(row_tops: &[f32]) -> f32 {
    row_tops.last().copied().unwrap_or(0.0)
}

#[allow(clippy::cast_precision_loss)]
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
#[allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    missing_docs
)]
mod tests {
    use super::*;

    #[test]
    fn visible_window_at_scroll_zero_returns_first_window() {
        // 15 in view + 15 overscan below. Nothing above.
        assert_eq!(
            visible_window(0.0, 100, 32.0, 480.0),
            VisibleWindow { first: 0, end: 30 }
        );
        assert_eq!(
            visible_window(1.0, 100, 32.0, 480.0),
            VisibleWindow { first: 0, end: 31 }
        );
    }

    #[test]
    fn visible_window_mid_list_pads_one_page_each_side() {
        // Geometric page is 50..65. Pad 15 → 35..80.
        assert_eq!(
            visible_window(1_600.0, 100, 32.0, 480.0),
            VisibleWindow { first: 35, end: 80 }
        );
    }

    #[test]
    fn one_page_jump_stays_inside_the_mounted_window() {
        let here = visible_window(0.0, 1_000, 32.0, 480.0);
        let next = visible_window(480.0, 1_000, 32.0, 480.0);
        assert!(next.first < here.end);
        assert!(here.first < next.end);
    }

    #[test]
    fn visible_window_clamps_to_data_length() {
        assert_eq!(
            visible_window(99_999.0, 100, 32.0, 480.0),
            VisibleWindow {
                first: 85,
                end: 100
            }
        );
    }

    #[test]
    fn empty_items_produces_empty_window() {
        assert_eq!(
            visible_window(0.0, 0, 32.0, 480.0),
            VisibleWindow { first: 0, end: 0 }
        );
    }

    #[test]
    fn variable_height_prefix_sum_locates_row() {
        let row_tops = variable_row_tops();

        assert_px_eq(prefix_row_top(&row_tops, 0), 0.0);
        assert_px_eq(prefix_row_top(&row_tops, 2), 80.0);
        assert_px_eq(prefix_row_top(&row_tops, 4), 140.0);
        assert_px_eq(total_height_variable(&row_tops), 200.0);
    }

    #[test]
    fn binary_search_finds_row_at_y() {
        let row_tops = variable_row_tops();

        assert_eq!(y_to_row(&row_tops, 85.0), 2);
        assert_eq!(y_to_row(&row_tops, 199.0), 4);
        assert_eq!(y_to_row(&row_tops, 0.0), 0);
    }

    #[test]
    fn variable_height_visible_window() {
        let row_tops = variable_row_tops();

        assert_eq!(
            visible_window_variable(85.0, &row_tops, 100.0),
            VisibleWindow { first: 0, end: 5 }
        );
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn variable_height_mid_list_pads_one_viewport() {
        // 20 rows × 20 px. Viewport 40 px → geometric 4..6 at y=80.
        let mut tops = [0.0; 21];
        for (i, top) in tops.iter_mut().enumerate() {
            *top = i as f32 * 20.0;
        }
        assert_eq!(
            visible_window_variable(80.0, &tops, 40.0),
            VisibleWindow { first: 2, end: 8 }
        );
    }

    fn variable_row_tops() -> [f32; 6] {
        [0.0, 30.0, 80.0, 100.0, 140.0, 200.0]
    }

    fn assert_px_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() <= f32::EPSILON);
    }
}
