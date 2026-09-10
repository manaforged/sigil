use super::super::coords::LineCol;
use super::super::rope::Rope;
use super::{Cursor, CursorSet};

impl CursorSet {
    pub fn move_horizontal(&mut self, rope: &Rope, steps: isize) {
        self.move_each(|c| {
            let new_head = step_horizontal(rope, c.head, steps);
            Cursor::at(new_head)
        });
    }

    pub fn extend_horizontal(&mut self, rope: &Rope, steps: isize) {
        let extender = |c: &Cursor| Cursor {
            anchor: c.anchor,
            head: step_horizontal(rope, c.head, steps),
        };
        self.move_each(extender);
    }

    pub fn move_vertical(&mut self, rope: &Rope, lines: isize) {
        self.move_each(|c| {
            let new_head = step_vertical(rope, c.head, lines);
            Cursor::at(new_head)
        });
    }

    pub fn add_above(&mut self, rope: &Rope) {
        self.add_relative_line(rope, -1);
    }

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

    pub fn move_to_line_start(&mut self, rope: &Rope) {
        self.move_each(|c| Cursor::at(line_start(rope, c.head)));
    }

    pub fn move_to_line_end(&mut self, rope: &Rope) {
        self.move_each(|c| Cursor::at(line_end(rope, c.head)));
    }

    pub fn move_to_buffer_start(&mut self) {
        self.set_primary(Cursor::at(0));
    }

    pub fn move_to_buffer_end(&mut self, rope: &Rope) {
        self.set_primary(Cursor::at(rope.len_bytes()));
    }
}

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
            break;
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

fn total_lines(rope: &Rope) -> usize {
    if rope.len_bytes() == 0 {
        1
    } else {
        rope.len_lines() + 1
    }
}

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
#[path = "motion_tests.rs"]
mod tests;
