//! Cursor motion APIs.
//!
//! All motion ops walk UTF-8 char boundaries — rope offsets are always
//! char boundaries (the rope panics on non-boundary inserts in debug
//! mode), so char-step motion is the safe baseline. CRLF pairs (`\r\n`)
//! are treated atomically so left/right motion never lands between
//! `\r` and `\n`.
//!
//! Full grapheme-cluster motion (combining marks, ZWJ sequences,
//! regional indicators) needs `unicode-segmentation`; that's a
//! follow-up. Today's motion stops at char boundaries, which matches
//! what the rope itself enforces.

use super::super::coords::LineCol;
use super::super::rope::Rope;
use super::{Cursor, CursorSet};

impl CursorSet {
    /// Move every cursor's head left (`steps < 0`) or right
    /// (`steps > 0`) by `steps.abs()` char boundaries. Anchors collapse
    /// to the new head (motion clears selection).
    ///
    /// CRLF is treated as a single stop.
    pub fn move_horizontal(&mut self, rope: &Rope, steps: isize) {
        self.move_each(|c| {
            let new_head = step_horizontal(rope, c.head, steps);
            Cursor::at(new_head)
        });
    }

    /// Extend every selection — anchor stays, head moves by `steps`
    /// char boundaries.
    pub fn extend_horizontal(&mut self, rope: &Rope, steps: isize) {
        let extender = |c: &Cursor| Cursor {
            anchor: c.anchor,
            head: step_horizontal(rope, c.head, steps),
        };
        self.move_each(extender);
    }

    /// Move every cursor's head up (`lines < 0`) or down (`lines > 0`)
    /// by `lines.abs()` lines. Column (UTF-8 byte column) is preserved
    /// across lines; if the target line is shorter, the head clamps to
    /// the line's end.
    ///
    /// Anchors collapse to the new head (motion clears selection).
    pub fn move_vertical(&mut self, rope: &Rope, lines: isize) {
        self.move_each(|c| {
            let new_head = step_vertical(rope, c.head, lines);
            Cursor::at(new_head)
        });
    }

    /// Add a caret cursor one line above (or below) every existing
    /// cursor at the same byte column. Out-of-range targets (above
    /// line 0 / below last line) are silently dropped. Coalesces
    /// after.
    pub fn add_above(&mut self, rope: &Rope) {
        self.add_relative_line(rope, -1);
    }

    /// See [`Self::add_above`].
    pub fn add_below(&mut self, rope: &Rope) {
        self.add_relative_line(rope, 1);
    }

    fn add_relative_line(&mut self, rope: &Rope, delta: isize) {
        let new_heads: Vec<usize> = self
            .cursors()
            .iter()
            .filter_map(|c| relative_line_head(rope, c.head, delta))
            .collect();
        for h in new_heads {
            self.add(Cursor::at(h));
        }
        self.coalesce();
    }

    /// Move every cursor's head to the start of its line.
    pub fn move_to_line_start(&mut self, rope: &Rope) {
        self.move_each(|c| Cursor::at(line_start(rope, c.head)));
    }

    /// Move every cursor's head to the end of its line (the position
    /// just before the trailing `\n`, or buffer-end if no newline).
    pub fn move_to_line_end(&mut self, rope: &Rope) {
        self.move_each(|c| Cursor::at(line_end(rope, c.head)));
    }

    /// Move every cursor to byte 0 (collapses to one cursor).
    pub fn move_to_buffer_start(&mut self) {
        self.set_primary(Cursor::at(0));
    }

    /// Move every cursor to buffer end (collapses to one cursor).
    pub fn move_to_buffer_end(&mut self, rope: &Rope) {
        self.set_primary(Cursor::at(rope.len_bytes()));
    }
}

// ---- Free helpers ------------------------------------------------------

/// Step `byte` left/right by `steps.abs()` char boundaries. CRLF is one
/// stop. Clamps to `[0, rope.len_bytes()]`.
pub(crate) fn step_horizontal(rope: &Rope, byte: usize, steps: isize) -> usize {
    let len = rope.len_bytes();
    let mut pos = byte.min(len);
    if steps == 0 {
        return pos;
    }
    let text = rope.to_string_lossy();
    let bytes = text.as_bytes();
    let mut remaining = steps.unsigned_abs();
    let forward = steps > 0;
    while remaining > 0 {
        let next = if forward {
            step_one_right(bytes, pos)
        } else {
            step_one_left(bytes, pos)
        };
        if next == pos {
            break; // hit boundary
        }
        pos = next;
        remaining -= 1;
    }
    pos
}

fn step_one_right(bytes: &[u8], pos: usize) -> usize {
    if pos >= bytes.len() {
        return pos;
    }
    let mut next = pos + 1;
    while next < bytes.len() && bytes.get(next).is_some_and(|b| !is_char_start(*b)) {
        next += 1;
    }
    // CRLF: if we just stepped over '\r' and '\n' follows, skip the '\n'.
    let here = bytes.get(pos).copied();
    let landed = bytes.get(next).copied();
    if here == Some(b'\r') && landed == Some(b'\n') {
        next += 1;
    }
    next
}

fn step_one_left(bytes: &[u8], pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut next = pos - 1;
    while next > 0 && bytes.get(next).is_some_and(|b| !is_char_start(*b)) {
        next -= 1;
    }
    // CRLF: if we just stepped onto '\n' and '\r' precedes, step one more.
    let landed = bytes.get(next).copied();
    let before = bytes.get(next.wrapping_sub(1)).copied();
    if next > 0 && landed == Some(b'\n') && before == Some(b'\r') {
        next -= 1;
    }
    next
}

const fn is_char_start(b: u8) -> bool {
    (b & 0b1100_0000) != 0b1000_0000
}

/// Step a head `byte` up/down by `lines.abs()` lines, preserving the
/// byte column when possible.
pub(crate) fn step_vertical(rope: &Rope, byte: usize, lines: isize) -> usize {
    let lc = rope.byte_to_linecol(super::super::coords::ByteIdx(byte));
    let target_line = saturating_apply_delta(lc.line, lines);
    let line_total = total_lines(rope);
    let clamped_line = target_line.min(line_total.saturating_sub(1));
    let len = line_text_len(rope, clamped_line);
    let col = lc.col.min(len);
    let bi = rope.linecol_to_byte(LineCol {
        line: clamped_line,
        col,
    });
    bi.0
}

/// Apply a signed delta to `line` saturating at 0; returns the new
/// (unsigned) line index without any cast-warnings.
const fn saturating_apply_delta(line: usize, delta: isize) -> usize {
    if delta >= 0 {
        line.saturating_add(delta.unsigned_abs())
    } else {
        line.saturating_sub(delta.unsigned_abs())
    }
}

fn relative_line_head(rope: &Rope, byte: usize, delta: isize) -> Option<usize> {
    let lc = rope.byte_to_linecol(super::super::coords::ByteIdx(byte));
    if delta < 0 && delta.unsigned_abs() > lc.line {
        return None;
    }
    let next_line = saturating_apply_delta(lc.line, delta);
    if next_line >= total_lines(rope) {
        return None;
    }
    let col = lc.col.min(line_text_len(rope, next_line));
    Some(
        rope.linecol_to_byte(LineCol {
            line: next_line,
            col,
        })
        .0,
    )
}

fn line_start(rope: &Rope, byte: usize) -> usize {
    let lc = rope.byte_to_linecol(super::super::coords::ByteIdx(byte));
    rope.linecol_to_byte(LineCol {
        line: lc.line,
        col: 0,
    })
    .0
}

fn line_end(rope: &Rope, byte: usize) -> usize {
    let lc = rope.byte_to_linecol(super::super::coords::ByteIdx(byte));
    let len = line_text_len(rope, lc.line);
    rope.linecol_to_byte(LineCol {
        line: lc.line,
        col: len,
    })
    .0
}

/// Number of logical lines in the rope.
///
/// Equal to `len_lines() + 1` for any non-empty rope: each `\n` ends a
/// line and the buffer-tail (whether content or an empty line implied
/// by a trailing `\n`) counts as the next line. An empty rope has one
/// (empty) line.
fn total_lines(rope: &Rope) -> usize {
    if rope.len_bytes() == 0 {
        1
    } else {
        rope.len_lines() + 1
    }
}

/// Length in bytes of the `line_idx`-th line's text, excluding any
/// trailing `\n`.
fn line_text_len(rope: &Rope, line_idx: usize) -> usize {
    let slice = rope.line(line_idx);
    let len = slice.len_bytes();
    if len == 0 {
        return 0;
    }
    let trailing_nl = slice
        .as_str()
        .is_some_and(|s| s.as_bytes().last() == Some(&b'\n'));
    if trailing_nl { len - 1 } else { len }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::super::Cursor;
    use super::*;

    #[test]
    fn move_horizontal_basic() {
        let rope = Rope::from_str("hello");
        let mut set = CursorSet::new(Cursor::at(0));
        set.move_horizontal(&rope, 3);
        assert_eq!(set.cursors()[0].head, 3);
        set.move_horizontal(&rope, -2);
        assert_eq!(set.cursors()[0].head, 1);
        // Past start clamps to 0.
        set.move_horizontal(&rope, -10);
        assert_eq!(set.cursors()[0].head, 0);
        // Past end clamps to len.
        set.move_horizontal(&rope, 99);
        assert_eq!(set.cursors()[0].head, 5);
    }

    #[test]
    fn move_horizontal_skips_crlf_atomically() {
        // "a\r\nb" — moving right from 'a' (pos 1) should jump over CRLF to 'b' (pos 3).
        let rope = Rope::from_str("a\r\nb");
        let mut set = CursorSet::new(Cursor::at(1));
        set.move_horizontal(&rope, 1);
        assert_eq!(set.cursors()[0].head, 3, "should skip CRLF as one stop");
        // Moving left from 'b' (3) should land back at 'a' end (1).
        set.move_horizontal(&rope, -1);
        assert_eq!(set.cursors()[0].head, 1);
    }

    #[test]
    fn move_horizontal_multibyte_char() {
        // "🦀x" — 🦀 is 4 bytes. One right from 0 lands at byte 4.
        let rope = Rope::from_str("🦀x");
        let mut set = CursorSet::new(Cursor::at(0));
        set.move_horizontal(&rope, 1);
        assert_eq!(set.cursors()[0].head, 4);
        set.move_horizontal(&rope, 1);
        assert_eq!(set.cursors()[0].head, 5);
    }

    #[test]
    fn move_vertical_preserves_byte_column() {
        // Three lines of equal width; moving down keeps column.
        let rope = Rope::from_str("hello\nworld\nrusty");
        let mut set = CursorSet::new(Cursor::at(3));
        set.move_vertical(&rope, 1);
        assert_eq!(set.cursors()[0].head, 6 + 3); // line 1 starts at 6, col 3 → 9
    }

    #[test]
    fn move_vertical_clamps_to_short_line() {
        // Line 1 is short (2 bytes); moving from col 4 on line 0 down should clamp.
        let rope = Rope::from_str("hello\nab\nworld");
        let mut set = CursorSet::new(Cursor::at(4)); // on line 0, col 4
        set.move_vertical(&rope, 1);
        // Line 1 starts at byte 6, has "ab" (len 2), col clamps to 2 → byte 8.
        assert_eq!(set.cursors()[0].head, 8);
    }

    #[test]
    fn add_below_preserves_column() {
        let rope = Rope::from_str("hello\nworld\nrusty");
        let mut set = CursorSet::new(Cursor::at(2)); // line 0, col 2
        set.add_below(&rope);
        let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
        // Original at byte 2, new at line 1 col 2 = byte 8.
        assert_eq!(heads, vec![2, 8]);
    }

    #[test]
    fn add_above_at_first_line_is_noop() {
        let rope = Rope::from_str("hello\nworld");
        let mut set = CursorSet::new(Cursor::at(2));
        set.add_above(&rope);
        assert_eq!(set.cursors().len(), 1);
    }

    #[test]
    fn move_to_line_start_and_end() {
        let rope = Rope::from_str("hello\nworld");
        let mut set = CursorSet::new(Cursor::at(8)); // middle of line 1
        set.move_to_line_start(&rope);
        assert_eq!(set.cursors()[0].head, 6);
        set.move_to_line_end(&rope);
        assert_eq!(set.cursors()[0].head, 11);
    }

    #[test]
    fn move_to_buffer_bounds() {
        let rope = Rope::from_str("hello\nworld");
        let mut set = CursorSet::new(Cursor::at(0));
        set.add_at(5);
        set.move_to_buffer_end(&rope);
        assert_eq!(set.cursors().len(), 1);
        assert_eq!(set.cursors()[0].head, 11);
        set.move_to_buffer_start();
        assert_eq!(set.cursors()[0].head, 0);
    }

    #[test]
    fn extend_horizontal_keeps_anchor() {
        let rope = Rope::from_str("hello");
        let mut set = CursorSet::new(Cursor::at(2));
        set.extend_horizontal(&rope, 2);
        let c = set.cursors()[0];
        assert_eq!(c.anchor, 2);
        assert_eq!(c.head, 4);
    }
}
