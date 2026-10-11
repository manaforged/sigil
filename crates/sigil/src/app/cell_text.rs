use std::cell::Cell;

use crate::geometry::{Dim, LogicalSize, Point, Rect, Size};
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::TextContent;
use crate::layout::{CaretGeometry, TextGeometry, TextMeasure};
use crate::style::TextStyle;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Copy, Default)]
pub enum CellWidth {
    #[default]
    Unicode,
    Scalar {
        joiner: usize,
        nonjoiner: usize,
        keycap: usize,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ambiguous {
    #[default]
    Narrow,
    Wide,
}

thread_local! {
    static CELL_WIDTH: Cell<CellWidth> = const { Cell::new(CellWidth::Unicode) };
    static AMBIGUOUS: Cell<Ambiguous> = const { Cell::new(Ambiguous::Narrow) };
}

pub fn set_cell_width(width: CellWidth) {
    CELL_WIDTH.set(width);
}

pub fn set_ambiguous(width: Ambiguous) {
    AMBIGUOUS.set(width);
}

fn char_width(ch: char) -> usize {
    match AMBIGUOUS.get() {
        Ambiguous::Narrow => UnicodeWidthChar::width(ch),
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

#[must_use]
pub fn cell_width(text: &str) -> usize {
    match CELL_WIDTH.get() {
        CellWidth::Unicode => match AMBIGUOUS.get() {
            Ambiguous::Narrow => UnicodeWidthStr::width(text),
            Ambiguous::Wide => UnicodeWidthStr::width_cjk(text),
        },
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
        let max_cells = width.and_then(|value| {
            if self.px <= 0.0 {
                return None;
            }
            let cells = (value / self.px).floor();
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "the positive cell count is bounded by the available terminal width"
            )]
            let cells = cells as usize;
            Some(cells.max(1))
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
        let byte = floor_boundary(text, position.byte_index);
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
}

impl TextGeometry for CellGeometry {
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
        let lo = floor_boundary(text, anchor.byte_index.min(focus.byte_index));
        let hi = floor_boundary(text, anchor.byte_index.max(focus.byte_index));
        let mut rects = Vec::new();
        for (line, &(start, end)) in ranges.iter().enumerate() {
            let text_selected = lo < end && hi > start;
            let newline_selected = hard_break_at(text, end) && lo <= end && hi > end;
            if !text_selected && !newline_selected {
                continue;
            }
            let from = lo.clamp(start, end);
            let to = hi.clamp(start, end);
            let line_width = cells_to_px(text.get(start..end).map_or(0, cell_width), self.px);
            let x = if text_selected {
                cells_to_px(text.get(start..from).map_or(0, cell_width), self.px)
            } else {
                line_width
            };
            let mut selected = if text_selected {
                cells_to_px(text.get(from..to).map_or(0, cell_width), self.px)
            } else {
                0.0
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

pub fn cell_text_lines(text: &str, max_cells: usize) -> Vec<(usize, usize)> {
    ranges(text, (max_cells > 0).then_some(max_cells))
}

fn ranges(text: &str, max_cells: Option<usize>) -> Vec<(usize, usize)> {
    let breaks = break_positions(text);
    let mut lines = Vec::new();
    for (start, end) in hard_lines(text) {
        lines.extend(wrap_range(text, start, end, max_cells, &breaks));
    }
    lines
}

fn hard_lines(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while let Some(ch) = text.get(index..).and_then(|rest| rest.chars().next()) {
        if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            out.push((start, index));
            index += ch.len_utf8();
            if ch == '\r' && text.get(index..).is_some_and(|rest| rest.starts_with('\n')) {
                index += 1;
            }
            start = index;
        } else {
            index += ch.len_utf8();
        }
    }
    if out.is_empty() {
        out.push((0, text.len()));
    } else {
        out.push((start, text.len()));
    }
    out
}

fn wrap_range(
    text: &str,
    start: usize,
    end: usize,
    max_cells: Option<usize>,
    breaks: &[usize],
) -> Vec<(usize, usize)> {
    if start == end || max_cells.is_none() {
        return vec![(start, end)];
    }
    let max_cells = max_cells.unwrap_or(usize::MAX);
    let mut out = Vec::new();
    let mut line_start = start;
    while line_start < end {
        let mut width = 0_usize;
        let mut fit_end = line_start;
        let mut break_end = None;
        for (offset, grapheme) in text[line_start..end].grapheme_indices(true) {
            let grapheme_end = line_start + offset + grapheme.len();
            let next = width.saturating_add(cell_width(grapheme));
            if fit_end > line_start && next > max_cells {
                break;
            }
            width = next;
            fit_end = grapheme_end;
            if breaks.binary_search(&grapheme_end).is_ok() {
                break_end = Some(grapheme_end);
            }
        }
        if fit_end == line_start {
            fit_end = text[line_start..end]
                .graphemes(true)
                .next()
                .map_or(end, |grapheme| line_start + grapheme.len());
        }
        let cut = if fit_end < end {
            break_end
                .filter(|value| *value > line_start)
                .unwrap_or(fit_end)
        } else {
            end
        };
        let rest = &text[cut..end];
        let cut = cut + rest.len() - rest.trim_start_matches(' ').len();
        out.push((line_start, cut));
        line_start = cut;
    }
    out
}

fn break_positions(text: &str) -> Vec<usize> {
    unicode_linebreak::linebreaks(text)
        .filter_map(|(index, opportunity)| {
            matches!(opportunity, unicode_linebreak::BreakOpportunity::Allowed).then_some(index)
        })
        .collect()
}

fn hard_break_at(text: &str, byte: usize) -> bool {
    text.get(byte..).is_some_and(|rest| {
        rest.starts_with('\n')
            || rest.starts_with('\r')
            || rest.starts_with('\u{2028}')
            || rest.starts_with('\u{2029}')
    })
}

fn cells_to_px(cells: usize, px: f32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "terminal cell counts stay within the representable pixel range"
    )]
    let cells = cells as f32;
    cells * px
}

fn floor_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

#[cfg(test)]
#[path = "cell_text_tests.rs"]
mod tests;
