use super::TextEdit;

#[test]
fn place_caret_sets_caret_and_collapses_selection() {
    let mut edit = TextEdit::new("hello world".to_owned());
    edit.select_all();
    assert!(edit.selection().is_some());
    edit.place_caret(3);
    assert_eq!(edit.caret(), 3);
    assert_eq!(edit.selection(), None, "click collapses the selection");
}

#[test]
fn place_caret_clamps_past_end_and_off_boundary() {
    let mut edit = TextEdit::new("café".to_owned());
    edit.place_caret(999);
    assert_eq!(edit.caret(), edit.len(), "past-end clamps to length");
    edit.place_caret(4);
    assert_eq!(edit.caret(), 3, "mid-codepoint snaps back to a boundary");
}

#[test]
fn extend_to_anchors_at_starting_caret_then_grows() {
    let mut edit = TextEdit::new("hello world".to_owned());
    edit.place_caret(2);
    edit.extend_to(6);
    assert_eq!(edit.caret(), 6);
    assert_eq!(edit.selection(), Some(2), "anchor stays at the drag origin");
    assert_eq!(edit.selected_text(), Some("llo "));
}

#[test]
fn extend_to_back_onto_anchor_collapses() {
    let mut edit = TextEdit::new("hello".to_owned());
    edit.place_caret(1);
    edit.extend_to(4);
    assert_eq!(edit.selection(), Some(1));
    edit.extend_to(1);
    assert_eq!(edit.selection(), None, "a zero-width drag collapses");
    assert_eq!(edit.caret(), 1);
}

#[test]
fn sync_external_clamps_caret_past_new_end() {
    let mut edit = TextEdit::new("hello world".to_owned());
    edit.place_caret(11);
    edit.sync_external("hi".to_owned());
    assert_eq!(edit.as_str(), "hi");
    assert_eq!(edit.caret(), 2, "caret clamps to the new (shorter) length");
}

#[test]
fn sync_external_snaps_caret_off_new_boundary() {
    let mut edit = TextEdit::new("world".to_owned());
    edit.place_caret(4);
    edit.sync_external("café".to_owned());
    assert_eq!(
        edit.caret(),
        3,
        "mid-codepoint offset snaps back to a boundary"
    );
}

#[test]
fn sync_external_is_a_no_op_when_buffer_already_matches() {
    let mut edit = TextEdit::new("hello".to_owned());
    edit.place_caret(3);
    edit.extend_to(5);
    edit.sync_external("hello".to_owned());
    assert_eq!(
        edit.caret(),
        5,
        "matching external value must not disturb caret"
    );
    assert_eq!(
        edit.selection(),
        Some(3),
        "matching external value must not disturb selection"
    );
}

#[test]
fn move_up_down_keeps_byte_column_across_lines() {
    let mut edit = TextEdit::new("abc\nde\nfghi".to_owned());
    edit.place_caret(6);
    edit.move_up();
    assert_eq!(edit.caret(), 2, "up keeps column 2 on line 0");
    edit.move_down();
    assert_eq!(edit.caret(), 6, "down returns to line 1 column 2");
    edit.move_down();
    assert_eq!(edit.caret(), 9, "down to line 2 column 2");
    edit.move_down();
    assert_eq!(edit.caret(), edit.len(), "past last line clamps to end");
}

#[test]
fn move_up_clamps_column_to_shorter_line_end() {
    let mut edit = TextEdit::new("a\nlongline".to_owned());
    edit.place_caret(6);
    edit.move_up();
    assert_eq!(edit.caret(), 1, "column clamps to the shorter line's end");
}

#[test]
fn page_up_down_jumps_ten_lines() {
    let buffer = (0..25)
        .map(|i| format!("l{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut edit = TextEdit::new(buffer);
    edit.place_caret(edit.len());
    edit.move_page_up();
    assert_eq!(edit.caret(), 59, "page up lands ten lines earlier");
    edit.move_page_down();
    assert_eq!(edit.caret(), edit.len(), "page down returns to the end");
    edit.place_caret(0);
    edit.move_page_up();
    assert_eq!(edit.caret(), 0, "page up from the first line stays at 0");
}

#[test]
fn move_up_from_first_line_goes_to_start() {
    let mut edit = TextEdit::new("abc\ndef".to_owned());
    edit.place_caret(2);
    edit.move_up();
    assert_eq!(edit.caret(), 0, "above the first line is byte 0");
}

#[test]
fn vertical_moves_never_split_multibyte_graphemes() {
    let mut edit = TextEdit::new("é\nab".to_owned());
    edit.place_caret(4);
    edit.move_up();
    let caret = edit.caret();
    assert!(
        caret == 0 || caret == 2,
        "target floors to a boundary, got {caret}"
    );
    assert!(edit.as_str().is_char_boundary(caret));
}

#[test]
fn shift_down_extends_selection_to_next_line() {
    let mut edit = TextEdit::new("ab\ncd".to_owned());
    edit.place_caret(1);
    edit.shift_down();
    assert_eq!(edit.caret(), 4, "caret lands same column on next line");
    assert_eq!(
        edit.selected_text(),
        Some("b\nc"),
        "selection spans the line break"
    );
}

#[test]
fn rtl_arrow_keys_move_visually() {
    let mut edit = TextEdit::new("אבג".to_owned());
    assert_eq!(edit.caret(), 6, "caret starts at logical end");
    edit.move_right();
    assert_eq!(edit.caret(), 4, "visual-right = logical back");
    edit.move_right();
    assert_eq!(edit.caret(), 2);
    edit.move_right();
    assert_eq!(
        edit.caret(),
        0,
        "reached the visual-right edge (logical start)"
    );
    edit.move_right();
    assert_eq!(edit.caret(), 0, "no move past the visual-right edge");
    edit.move_left();
    assert_eq!(edit.caret(), 2, "visual-left = logical forward");
}

#[test]
fn ltr_arrow_keys_stay_logical() {
    let mut edit = TextEdit::new("abc".to_owned());
    edit.place_caret(1);
    edit.move_right();
    assert_eq!(edit.caret(), 2);
    edit.move_left();
    assert_eq!(edit.caret(), 1);
}

#[test]
fn word_motion_and_deletion_follow_word_boundaries() {
    let mut edit = TextEdit::new("fix the  frame_loop, then ship".to_owned());
    edit.move_word_left();
    assert_eq!(edit.caret(), 26, "back over `ship`");
    edit.move_word_left();
    assert_eq!(edit.caret(), 21, "back over `then`");
    edit.move_word_right();
    assert_eq!(edit.caret(), 25, "forward to the end of `then`");
    edit.place_caret(19);
    edit.delete_word_backward();
    assert_eq!(
        edit.as_str(),
        "fix the  , then ship",
        "`frame_loop` is one word"
    );
    edit.delete_word_forward();
    assert_eq!(
        edit.as_str(),
        "fix the   ship",
        "forward delete eats `, then`"
    );
    edit.undo();
    assert_eq!(edit.as_str(), "fix the  , then ship");
}

#[test]
fn line_deletion_stops_at_newlines() {
    let mut edit = TextEdit::new("first line\nsecond line\nthird".to_owned());
    edit.place_caret(18);
    edit.delete_to_line_start();
    assert_eq!(edit.as_str(), "first line\nline\nthird");
    assert_eq!(edit.caret(), 11);
    edit.delete_to_line_end();
    assert_eq!(edit.as_str(), "first line\n\nthird");
    edit.select_all();
    edit.delete_to_line_end();
    assert_eq!(edit.as_str(), "", "a selection wins over the line span");
}
