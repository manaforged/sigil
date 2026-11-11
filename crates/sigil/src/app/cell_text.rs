use std::cell::Cell;

use crate::geometry::{Dim, LogicalSize, Point, Rect, Size};
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::TextContent;
use crate::layout::{CaretGeometry, TextMeasure};
use crate::style::TextStyle;
use unicode_segmentation::UnicodeSegmentation;
#[cfg(terminal_host)]
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

mod wrap;

#[cfg(feature = "terminal")]
pub(crate) use wrap::cell_text_lines;
use wrap::{hard_break_at, ranges};

kittest_pub! {
    /// Selects how the terminal host counts cell width: Unicode East Asian width, or fixed widths for joiners, non-joiners, and keycaps.
    #[derive(Clone, Copy, Debug, Default)]
    pub enum CellWidth {
        /// The unicode cell width.
        #[default]
        Unicode,
        /// The scalar cell width.
        #[cfg(terminal_host)]
        Scalar {
            /// The joiner.
            joiner: usize,
            /// The nonjoiner.
            nonjoiner: usize,
            /// The keycap.
            keycap: usize,
        },
    }
}

macro_rules! ambiguous {
    ($vis:vis) => {
        /// How wide East Asian ambiguous characters are in cells.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        #[non_exhaustive]
        $vis enum Ambiguous {
            /// The narrow ambiguous.
            #[default]
            Narrow,
            /// The wide ambiguous.
            #[cfg(terminal_host)]
            Wide,
        }
    };
}

#[cfg(terminal_host)]
ambiguous!(pub);
#[cfg(not(terminal_host))]
ambiguous!(pub(crate));

thread_local! {
    static CELL_WIDTH: Cell<CellWidth> = const { Cell::new(CellWidth::Unicode) };
    static AMBIGUOUS: Cell<Ambiguous> = const { Cell::new(Ambiguous::Narrow) };
}

#[cfg(terminal_host)]
kittest_pub! {
/// Sets how text width in cells is measured.
#[doc(hidden)]
pub fn set_cell_width(width: CellWidth) {
    CELL_WIDTH.set(width);
}
}

#[cfg(terminal_host)]
kittest_pub! {
/// Sets the width of East Asian ambiguous characters.
#[doc(hidden)]
pub fn set_ambiguous(width: Ambiguous) {
    AMBIGUOUS.set(width);
}
}

#[cfg(terminal_host)]
fn char_width(ch: char) -> usize {
    match AMBIGUOUS.get() {
        Ambiguous::Narrow => UnicodeWidthChar::width(ch),
        #[cfg(terminal_host)]
        Ambiguous::Wide => UnicodeWidthChar::width_cjk(ch),
    }
    .unwrap_or(0)
}

#[must_use]
#[cfg(feature = "terminal")]
pub(crate) fn cell_glyph(text: &str) -> &str {
    if text.chars().any(char::is_control) {
        " "
    } else {
        text
    }
}

/// Returns the width of `text` in terminal cells.
#[must_use]
pub fn cell_width(text: &str) -> usize {
    match CELL_WIDTH.get() {
        CellWidth::Unicode => match AMBIGUOUS.get() {
            Ambiguous::Narrow => UnicodeWidthStr::width(text),
            #[cfg(terminal_host)]
            Ambiguous::Wide => UnicodeWidthStr::width_cjk(text),
        },
        #[cfg(terminal_host)]
        CellWidth::Scalar {
            joiner,
            nonjoiner,
            keycap,
        } => text
            .chars()
            .map(|ch| match ch {
                '\u{200d}' => joiner,
                '\u{200c}' => nonjoiner,
                '\u{20e3}' => keycap,
                _ => char_width(ch),
            })
            .sum(),
    }
}

pub(super) struct CellGeometry {
    px: f32,
}

impl CellGeometry {
    #[must_use]
    pub(super) const fn new(px: f32) -> Self {
        Self { px }
    }

    fn ranges(&self, text: &str, width: Option<f32>) -> Vec<(usize, usize)> {
        let max_cells = width.filter(|_| self.px > 0.0).map(|value| {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "the positive cell count is bounded by the available terminal width"
            )]
            let cells = (value / self.px).floor() as usize;
            cells.max(1)
        });
        ranges(text, max_cells)
    }

    fn line_for_byte(ranges: &[(usize, usize)], byte: usize, affinity: TextAffinity) -> usize {
        for (line, &(_, end)) in ranges.iter().enumerate() {
            if byte < end {
                return line;
            }
            if ranges
                .get(line + 1)
                .is_some_and(|(next, _)| byte < *next && *next > end)
            {
                return line;
            }
            if byte == end
                && (affinity == TextAffinity::Upstream
                    || ranges.get(line + 1).is_none_or(|(next, _)| *next != end))
            {
                return line;
            }
        }
        ranges.len().saturating_sub(1)
    }

    fn hit_range(&self, text: &str, range: (usize, usize), x: f32) -> HitPosition {
        let mut left = 0.0;
        let x = x.max(0.0);
        let source = text.get(range.0..range.1).unwrap_or_default();
        for (offset, grapheme) in source.grapheme_indices(true) {
            let width = cells_to_px(cell_width(grapheme), self.px);
            if x < width.mul_add(0.5, left) {
                return HitPosition {
                    byte_index: range.0 + offset,
                    affinity: TextAffinity::Downstream,
                };
            }
            left += width;
        }
        HitPosition {
            byte_index: range.1,
            affinity: TextAffinity::Upstream,
        }
    }

    fn caret_for(
        &self,
        text: &str,
        ranges: &[(usize, usize)],
        position: HitPosition,
    ) -> CaretGeometry {
        let byte = text.floor_char_boundary(position.byte_index);
        let line = Self::line_for_byte(ranges, byte, position.affinity);
        let range = ranges.get(line).copied().unwrap_or((0, 0));
        let end = byte.clamp(range.0, range.1);
        let x = cells_to_px(text.get(range.0..end).map_or(0, cell_width), self.px);
        CaretGeometry {
            x,
            top: cells_to_px(line, self.px),
            height: self.px.max(1.0),
        }
    }

    fn hit_wrapped_at(&self, text: &str, ranges: &[(usize, usize)], x: f32, y: f32) -> HitPosition {
        let row = if self.px > 0.0 && y > 0.0 {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "terminal row coordinates are bounded by the measured cell grid"
            )]
            let row = (y / self.px).floor() as usize;
            row
        } else {
            0
        };
        let line = row.min(ranges.len().saturating_sub(1));
        self.hit_range(text, ranges.get(line).copied().unwrap_or((0, 0)), x)
    }
}

impl TextMeasure for CellGeometry {
    fn measure(&self, content: &TextContent, space: Size) -> LogicalSize {
        let text = content.resolve();
        if matches!(space.width, Dim::MinContent) {
            let longest = text.split_whitespace().map(cell_width).max().unwrap_or(0);
            return LogicalSize::new(cells_to_px(longest, self.px), self.px);
        }
        let width = match space.width {
            Dim::Fixed(width) => Some(width),
            Dim::MinContent | Dim::MaxContent => None,
        };
        let ranges = self.ranges(&text, width);
        let cells = ranges
            .iter()
            .map(|(start, end)| text.get(*start..*end).map_or(0, cell_width))
            .max()
            .unwrap_or(0);
        let wide = cells_to_px(cells, self.px);
        LogicalSize::new(
            width.map_or(wide, |width| wide.min(width.max(self.px))),
            cells_to_px(ranges.len().max(1), self.px),
        )
    }

    fn hit(&self, text: &str, _style: &TextStyle, width: Option<f32>, point: Point) -> HitPosition {
        self.hit_wrapped_at(text, &self.ranges(text, width), point.x, point.y)
    }

    fn caret(
        &self,
        text: &str,
        _style: &TextStyle,
        width: Option<f32>,
        position: HitPosition,
    ) -> CaretGeometry {
        self.caret_for(text, &self.ranges(text, width), position)
    }

    fn selection_rects(
        &self,
        text: &str,
        _style: &TextStyle,
        width: Option<f32>,
        (anchor, focus): (HitPosition, HitPosition),
    ) -> Vec<Rect> {
        let ranges = self.ranges(text, width);
        let lo = text.floor_char_boundary(anchor.byte_index.min(focus.byte_index));
        let hi = text.floor_char_boundary(anchor.byte_index.max(focus.byte_index));
        let mut rects = Vec::new();
        for (line, &(start, end)) in ranges.iter().enumerate() {
            let text_selected = lo < end && hi > start;
            let newline_selected = hard_break_at(text, end) && lo <= end && hi > end;
            if !text_selected && !newline_selected {
                continue;
            }
            let from = lo.clamp(start, end);
            let to = hi.clamp(start, end);
            let span =
                |a: usize, b: usize| cells_to_px(text.get(a..b).map_or(0, cell_width), self.px);
            let (x, mut selected) = if text_selected {
                (span(start, from), span(from, to))
            } else {
                (span(start, end), 0.0)
            };
            if newline_selected {
                selected += self.px;
            }
            if selected > 0.0 {
                rects.push(Rect::new(
                    Point::new(x, cells_to_px(line, self.px)),
                    selected,
                    self.px.max(1.0),
                ));
            }
        }
        rects
    }
}

/// Cuts `text` to the longest grapheme prefix that fits in `cells` cells.
#[must_use]
#[cfg(feature = "terminal")]
pub(crate) fn clip_cells(text: &str, cells: usize) -> &str {
    let mut used = 0_usize;
    let end = text.grapheme_indices(true).find_map(|(index, grapheme)| {
        used = used.saturating_add(cell_width(grapheme));
        (used > cells).then_some(index)
    });
    &text[..end.unwrap_or(text.len())]
}

/// Shortens `text` to `width` cells with an ellipsis at the end.
#[must_use]
#[cfg(feature = "terminal")]
pub(crate) fn ellipsize_cells(text: &str, width: usize) -> String {
    if cell_width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    format!("{}…", clip_cells(text, width - 1))
}

/// Shortens `text` to `width` cells with an ellipsis at the start.
#[must_use]
#[cfg(test)]
pub(crate) fn ellipsize_cells_start(text: &str, width: usize) -> String {
    if cell_width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut used = 0;
    let start = text
        .grapheme_indices(true)
        .rev()
        .find_map(|(index, grapheme)| {
            used += cell_width(grapheme);
            (used > width - 1).then_some(index + grapheme.len())
        });
    format!("…{}", &text[start.unwrap_or(0)..])
}

/// Cuts or pads `text` to exactly `width` cells.
#[must_use]
#[cfg(test)]
pub(crate) fn fit_cells(text: &str, width: usize) -> String {
    let mut out = ellipsize_cells(text, width);
    let pad = width.saturating_sub(cell_width(&out));
    out.extend(std::iter::repeat_n(' ', pad));
    out
}

fn cells_to_px(cells: usize, px: f32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "terminal cell counts stay within the representable pixel range"
    )]
    let cells = cells as f32;
    cells * px
}

#[cfg(test)]
#[path = "cell_text_tests.rs"]
mod tests;
