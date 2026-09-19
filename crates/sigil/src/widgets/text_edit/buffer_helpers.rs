use unicode_segmentation::UnicodeSegmentation;

pub(super) fn clamp_to_len_boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .rfind(|index| *index <= byte)
        .unwrap_or(0)
}

pub(super) const fn selection_range(anchor: usize, caret: usize) -> (usize, usize) {
    if anchor < caret {
        (anchor, caret)
    } else {
        (caret, anchor)
    }
}

pub(super) fn previous_boundary(buffer: &str, caret: usize) -> Option<usize> {
    if caret == 0 {
        return None;
    }
    buffer[..caret]
        .grapheme_indices(true)
        .next_back()
        .map(|(index, _)| index)
}

pub(super) fn next_boundary(buffer: &str, caret: usize) -> Option<usize> {
    if caret >= buffer.len() {
        return None;
    }
    buffer[caret..]
        .grapheme_indices(true)
        .nth(1)
        .map_or(Some(buffer.len()), |(index, _)| Some(caret + index))
}

pub(super) fn line_start(buffer: &str, caret: usize) -> usize {
    buffer[..caret].rfind('\n').map_or(0, |index| index + 1)
}

pub(super) fn line_end(buffer: &str, caret: usize) -> usize {
    buffer[caret..]
        .find('\n')
        .map_or(buffer.len(), |index| caret + index)
}

pub(super) fn word_start_before(buffer: &str, caret: usize) -> usize {
    let mut chars = buffer[..caret].grapheme_indices(true).rev().peekable();
    let mut start = caret;
    while let Some((index, ch)) = chars.peek().copied() {
        if word(ch) {
            break;
        }
        start = index;
        chars.next();
    }
    while let Some((index, ch)) = chars.peek().copied() {
        if !word(ch) {
            break;
        }
        start = index;
        chars.next();
    }
    start
}

pub(super) fn word_end_after(buffer: &str, caret: usize) -> usize {
    let mut end = caret;
    let mut chars = buffer[caret..].grapheme_indices(true).peekable();
    while let Some((index, ch)) = chars.peek().copied() {
        if word(ch) {
            break;
        }
        end = caret + index + ch.len();
        chars.next();
    }
    while let Some((index, ch)) = chars.peek().copied() {
        if !word(ch) {
            break;
        }
        end = caret + index + ch.len();
        chars.next();
    }
    end
}

fn word(text: &str) -> bool {
    text.chars().any(|ch| ch.is_alphanumeric() || ch == '_')
}
