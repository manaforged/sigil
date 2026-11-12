use std::cell::OnceCell;

use unicode_segmentation::UnicodeSegmentation;

use super::cell_width;

/// Wraps text to `max_cells` terminal cells and returns the byte range of each line.
#[cfg(feature = "terminal")]
pub(crate) fn cell_text_lines(text: &str, max_cells: usize) -> Vec<(usize, usize)> {
    ranges(text, (max_cells > 0).then_some(max_cells))
}

pub(super) fn ranges(text: &str, max_cells: Option<usize>) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let Some(max_cells) = max_cells else {
        for_each_hard_line(text, |start, end| lines.push((start, end)));
        return lines;
    };
    let breaks = OnceCell::new();
    for_each_hard_line(text, |start, end| {
        push_wrapped(text, (start, end), max_cells, &breaks, |ls, le| {
            lines.push((ls, le));
        });
    });
    lines
}

pub(super) fn measure_lines(text: &str, max_cells: Option<usize>) -> (usize, usize) {
    let Some(max_cells) = max_cells else {
        let (mut wide, mut count) = (0, 0);
        for_each_hard_line(text, |start, end| {
            wide = wide.max(cell_width(&text[start..end]));
            count += 1;
        });
        return (wide, count);
    };
    if text.is_ascii()
        && text.len() <= max_cells
        && !text.bytes().any(|b| matches!(b, b'\n' | b'\r'))
    {
        return (cell_width(text), 1);
    }
    let breaks = OnceCell::new();
    let (mut wide, mut count) = (0, 0);
    for_each_hard_line(text, |start, end| {
        push_wrapped(text, (start, end), max_cells, &breaks, |ls, le| {
            wide = wide.max(cell_width(&text[ls..le]));
            count += 1;
        });
    });
    (wide, count)
}

fn for_each_hard_line(text: &str, mut visit: impl FnMut(usize, usize)) {
    let mut start = 0;
    let mut index = 0;
    while let Some(ch) = text.get(index..).and_then(|rest| rest.chars().next()) {
        if HARD_BREAKS.contains(&ch) {
            visit(start, index);
            index += ch.len_utf8();
            if ch == '\r' && text.get(index..).is_some_and(|rest| rest.starts_with('\n')) {
                index += 1;
            }
            start = index;
        } else {
            index += ch.len_utf8();
        }
    }
    visit(start, text.len());
}

fn push_wrapped(
    text: &str,
    (start, end): (usize, usize),
    max_cells: usize,
    breaks: &OnceCell<Vec<usize>>,
    mut push: impl FnMut(usize, usize),
) {
    if start == end {
        push(start, end);
        return;
    }
    let mut line_start = start;
    while line_start < end {
        let slice = text.get(line_start..end).unwrap_or_default();
        let fit_end = line_start + fit(slice, max_cells).max(1).min(slice.len());
        let cut = if fit_end < end {
            let breaks = breaks.get_or_init(|| break_positions(text));
            breaks
                .get(breaks.partition_point(|at| *at <= fit_end).wrapping_sub(1))
                .copied()
                .filter(|at| *at > line_start)
                .unwrap_or(fit_end)
        } else {
            end
        };
        let rest = text.get(cut..end).unwrap_or_default();
        let cut = cut + rest.len() - rest.trim_start_matches(' ').len();
        push(line_start, cut);
        line_start = cut;
    }
}

fn fit(slice: &str, max_cells: usize) -> usize {
    let mut width = 0_usize;
    let mut fit = 0_usize;
    let mut take = |grapheme: &str| {
        let next = width.saturating_add(cell_width(grapheme));
        if fit > 0 && next > max_cells {
            return false;
        }
        width = next;
        fit += grapheme.len();
        true
    };
    if slice.is_ascii() {
        let _ = (0..slice.len()).all(|at| slice.get(at..=at).is_some_and(&mut take));
    } else {
        let _ = slice.graphemes(true).all(take);
    }
    fit
}

fn break_positions(text: &str) -> Vec<usize> {
    unicode_linebreak::linebreaks(text)
        .filter_map(|(index, opportunity)| {
            matches!(opportunity, unicode_linebreak::BreakOpportunity::Allowed).then_some(index)
        })
        .collect()
}

const HARD_BREAKS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];

pub(super) fn hard_break_at(text: &str, byte: usize) -> bool {
    text.get(byte..)
        .is_some_and(|rest| rest.starts_with(HARD_BREAKS))
}
