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
    set.move_horizontal(&rope, -10);
    assert_eq!(set.cursors()[0].head, 0);
    set.move_horizontal(&rope, 99);
    assert_eq!(set.cursors()[0].head, 5);
}

#[test]
fn move_horizontal_skips_crlf_atomically() {
    let rope = Rope::from_str("a\r\nb");
    let mut set = CursorSet::new(Cursor::at(1));
    set.move_horizontal(&rope, 1);
    assert_eq!(set.cursors()[0].head, 3, "should skip CRLF as one stop");
    set.move_horizontal(&rope, -1);
    assert_eq!(set.cursors()[0].head, 1);
}

#[test]
fn move_horizontal_multibyte_char() {
    let rope = Rope::from_str("🦀x");
    let mut set = CursorSet::new(Cursor::at(0));
    set.move_horizontal(&rope, 1);
    assert_eq!(set.cursors()[0].head, 4);
    set.move_horizontal(&rope, 1);
    assert_eq!(set.cursors()[0].head, 5);
}

#[test]
fn move_vertical_preserves_byte_column() {
    let rope = Rope::from_str("hello\nworld\nrusty");
    let mut set = CursorSet::new(Cursor::at(3));
    set.move_vertical(&rope, 1);
    assert_eq!(set.cursors()[0].head, 6 + 3);
}

#[test]
fn move_vertical_clamps_to_short_line() {
    let rope = Rope::from_str("hello\nab\nworld");
    let mut set = CursorSet::new(Cursor::at(4));
    set.move_vertical(&rope, 1);
    assert_eq!(set.cursors()[0].head, 8);
}

#[test]
fn add_below_preserves_column() {
    let rope = Rope::from_str("hello\nworld\nrusty");
    let mut set = CursorSet::new(Cursor::at(2));
    set.add_below(&rope);
    let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
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
    let mut set = CursorSet::new(Cursor::at(8));
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
