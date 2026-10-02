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

pub(super) fn row_px(px: f32) -> f32 {
    if px.is_finite() && px > 0.0 {
        crate::layout::snap::track(px)
    } else {
        crate::layout::snap::line(DEFAULT_ROW_HEIGHT)
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

#[derive(Clone, Copy, Debug)]
pub(super) enum RowHeights<'a> {
    Fixed { rows: usize, height: f32 },
    Tops(&'a [f32]),
}

pub(super) fn visible_window(
    scroll_y: f32,
    heights: RowHeights<'_>,
    viewport_height: f32,
    overscan: usize,
) -> VisibleWindow {
    let total = heights.rows();
    if total == 0 {
        return VisibleWindow { first: 0, end: 0 };
    }
    let viewport_height = positive_px(viewport_height, DEFAULT_VIEWPORT_HEIGHT);
    let content_height = heights.total();
    let scroll_y = finite_scroll_y(scroll_y);
    let (vis_first, vis_end) = if scroll_y >= content_height {
        (total, total)
    } else {
        let viewport_bottom = (scroll_y + viewport_height).min(content_height);
        (
            heights.row_at(scroll_y).min(total),
            heights.first_at_or_after(viewport_bottom).min(total),
        )
    };
    let first = heights.pad_start(vis_first, viewport_height, overscan);
    let end = heights
        .pad_end(vis_end, viewport_height, overscan)
        .min(total);
    VisibleWindow {
        first,
        end: end.max(first),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
impl RowHeights<'_> {
    pub(super) fn fixed(rows: usize, height: f32) -> Self {
        Self::Fixed {
            rows,
            height: positive_px(height, DEFAULT_ROW_HEIGHT),
        }
    }

    pub(super) fn rows(self) -> usize {
        match self {
            Self::Fixed { rows, .. } => rows,
            Self::Tops(tops) => tops.len().saturating_sub(1),
        }
    }

    pub(super) fn total(self) -> f32 {
        self.top(self.rows())
    }

    pub(super) fn top(self, index: usize) -> f32 {
        match self {
            Self::Fixed { height, .. } => index as f32 * height,
            Self::Tops(tops) => tops
                .get(index)
                .or_else(|| tops.last())
                .copied()
                .unwrap_or(0.0),
        }
    }

    pub(super) fn span(self, index: usize) -> (f32, f32) {
        let top = self.top(index);
        (top, self.top(index.saturating_add(1)) - top)
    }

    pub(super) fn row_at(self, y: f32) -> usize {
        let last = self.rows().saturating_sub(1);
        let y = finite_scroll_y(y);
        match self {
            Self::Fixed { height, .. } => ((y / height).floor() as usize).min(last),
            Self::Tops(tops) => tops
                .partition_point(|top| *top <= y)
                .saturating_sub(1)
                .min(last),
        }
    }

    fn first_at_or_after(self, y: f32) -> usize {
        let y = finite_scroll_y(y);
        let at = match self {
            Self::Fixed { height, .. } => (y / height).ceil() as usize,
            Self::Tops(tops) => tops.partition_point(|top| *top < y),
        };
        at.min(self.rows())
    }

    fn pad_start(self, first: usize, viewport_height: f32, overscan: usize) -> usize {
        match self {
            Self::Fixed { height, .. } => {
                first.saturating_sub(pad_rows(viewport_height, height, overscan))
            }
            Self::Tops(_) => {
                let pad_px = viewport_height * overscan as f32;
                self.row_at((self.top(first) - pad_px).max(0.0))
            }
        }
    }

    fn pad_end(self, end: usize, viewport_height: f32, overscan: usize) -> usize {
        match self {
            Self::Fixed { height, .. } => {
                end.saturating_add(pad_rows(viewport_height, height, overscan))
            }
            Self::Tops(_) => {
                let pad_px = viewport_height * overscan as f32;
                self.first_at_or_after(self.top(end) + pad_px)
            }
        }
    }
}

pub(super) const fn covers(
    mounted: VisibleWindow,
    visible: VisibleWindow,
    margin: usize,
    total: usize,
) -> bool {
    let top = mounted.first == 0 || visible.first >= mounted.first.saturating_add(margin);
    let bottom = mounted.end >= total || visible.end.saturating_add(margin) <= mounted.end;
    mounted.end <= total && top && bottom
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
