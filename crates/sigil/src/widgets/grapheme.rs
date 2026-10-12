#![allow(clippy::redundant_pub_crate)]

use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn byte_at_col(text: &str, col: usize) -> usize {
    text.grapheme_indices(true)
        .nth(col)
        .map_or(text.len(), |(index, _)| index)
}

pub(crate) fn col_at_byte(text: &str, byte: usize) -> usize {
    let byte = clamp_to_len_boundary(text, byte);
    text[..byte].graphemes(true).count()
}

pub(crate) fn col_count(text: &str) -> usize {
    text.graphemes(true).count()
}

pub(crate) fn col_bytes(text: &str, sorted_cols: &[usize]) -> Vec<usize> {
    let mut starts = text.grapheme_indices(true).map(|(index, _)| index);
    let mut col = 0;
    let mut byte = starts.next().unwrap_or(text.len());
    let mut out = Vec::with_capacity(sorted_cols.len());
    for &want in sorted_cols {
        while col < want {
            byte = starts.next().unwrap_or(text.len());
            col += 1;
        }
        out.push(byte);
    }
    out
}

pub(crate) fn clamp_to_len_boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .rfind(|index| *index <= byte)
        .unwrap_or(0)
}

pub(crate) const fn selection_range(anchor: usize, caret: usize) -> (usize, usize) {
    if anchor < caret {
        (anchor, caret)
    } else {
        (caret, anchor)
    }
}

pub(crate) fn previous_boundary(buffer: &str, caret: usize) -> Option<usize> {
    if caret == 0 {
        return None;
    }
    buffer[..caret]
        .grapheme_indices(true)
        .next_back()
        .map(|(index, _)| index)
}

pub(crate) fn next_boundary(buffer: &str, caret: usize) -> Option<usize> {
    if caret >= buffer.len() {
        return None;
    }
    buffer[caret..]
        .grapheme_indices(true)
        .nth(1)
        .map_or(Some(buffer.len()), |(index, _)| Some(caret + index))
}

pub(crate) fn line_start(buffer: &str, caret: usize) -> usize {
    buffer[..caret].rfind('\n').map_or(0, |index| index + 1)
}

pub(crate) fn line_end(buffer: &str, caret: usize) -> usize {
    buffer[caret..]
        .find('\n')
        .map_or(buffer.len(), |index| caret + index)
}

pub(crate) fn word_start_before(buffer: &str, caret: usize) -> usize {
    let mut graphemes = buffer[..caret].grapheme_indices(true).rev().peekable();
    let mut start = caret;
    for want in [false, true] {
        while let Some((index, _)) = graphemes.next_if(|(_, text)| word(text) == want) {
            start = index;
        }
    }
    start
}

pub(crate) fn word_end_after(buffer: &str, caret: usize) -> usize {
    let mut graphemes = buffer[caret..].grapheme_indices(true).peekable();
    let mut end = caret;
    for want in [false, true] {
        while let Some((index, text)) = graphemes.next_if(|(_, text)| word(text) == want) {
            end = caret + index + text.len();
        }
    }
    end
}

fn word(text: &str) -> bool {
    text.chars().any(|ch| ch.is_alphanumeric() || ch == '_')
}
