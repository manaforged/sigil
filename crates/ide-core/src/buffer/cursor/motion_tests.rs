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
