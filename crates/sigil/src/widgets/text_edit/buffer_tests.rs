use super::TextEdit;

#[test]
fn editing_keeps_graphemes_whole() {
    for grapheme in ["👍🏽", "e\u{301}", "👩‍💻", "🇳🇿"] {
        let text = format!("A{grapheme}B");
        let mut edit = TextEdit::new_at(text.clone(), usize::MAX);
        edit.move_left();
        edit.backspace();
        assert_eq!(edit.as_str(), "AB");
        assert!(edit.undo());
        assert_eq!(edit.as_str(), text);
        edit.move_left();
        assert_eq!(edit.caret(), 1);
        edit.delete_forward();
        assert_eq!(edit.as_str(), "AB");
        let mut selection = TextEdit::new_at(text, usize::MAX);
        selection.place_caret(1 + grapheme.len() - 1);
        assert_eq!(selection.caret(), 1);
        selection.shift_right();
        assert_eq!(selection.selected_text(), Some(grapheme));
    }
}

#[test]
fn joining_lines_and_inserting_before_marks_keeps_a_grapheme_boundary() {
    let mut edit = TextEdit::new_at("e\n\u{301}".to_string(), 2);
    edit.backspace();
    assert_eq!(edit.as_str(), "e\u{301}");
    assert_eq!(edit.caret(), 0);
    edit.delete_forward();
    assert!(edit.is_empty());
    let mut insertion = TextEdit::new_at("\u{301}".to_string(), 0);
    insertion.insert_str("e");
    assert_eq!(insertion.caret(), "e\u{301}".len());
    insertion.backspace();
    assert!(insertion.is_empty());
    let mut replacement = TextEdit::new_at("e\n\u{301}".to_string(), 1);
    replacement.extend_to(2);
    replacement.insert_str("x");
    assert_eq!(replacement.as_str(), "ex\u{301}");
    assert_eq!(replacement.caret(), replacement.len());
}

#[test]
fn word_movement_preserves_combining_characters() {
    let mut words = TextEdit::new_at("e\u{301} next".to_string(), 0);
    words.move_word_right();
    assert_eq!(words.caret(), "e\u{301}".len());
    words.delete_word_backward();
    assert_eq!(words.as_str(), " next");
}

#[test]
fn place_caret_sets_caret_and_collapses_selection() {
    let mut edit = TextEdit::new_at("hello world".to_owned(), usize::MAX);
    edit.select_all();
    assert!(edit.selection().is_some());
    edit.place_caret(3);
    assert_eq!(edit.caret(), 3);
    assert_eq!(edit.selection(), None, "click collapses the selection");
}

#[test]
fn place_caret_clamps_past_end_and_off_boundary() {
    let mut edit = TextEdit::new_at("café".to_owned(), usize::MAX);
    edit.place_caret(999);
    assert_eq!(edit.caret(), edit.len(), "past-end clamps to length");
    edit.place_caret(4);
    assert_eq!(edit.caret(), 3, "mid-codepoint snaps back to a boundary");
}

#[test]
fn extend_to_anchors_at_starting_caret_then_grows() {
    let mut edit = TextEdit::new_at("hello world".to_owned(), usize::MAX);
    edit.place_caret(2);
    edit.extend_to(6);
    assert_eq!(edit.caret(), 6);
    assert_eq!(edit.selection(), Some(2), "anchor stays at the drag origin");
    assert_eq!(edit.selected_text(), Some("llo "));
}

#[test]
fn extend_to_back_onto_anchor_collapses() {
    let mut edit = TextEdit::new_at("hello".to_owned(), usize::MAX);
    edit.place_caret(1);
    edit.extend_to(4);
    assert_eq!(edit.selection(), Some(1));
    edit.extend_to(1);
    assert_eq!(edit.selection(), None, "a zero-width drag collapses");
    assert_eq!(edit.caret(), 1);
}

#[test]
fn sync_external_clamps_caret_past_new_end() {
    let mut edit = TextEdit::new_at("hello world".to_owned(), usize::MAX);
    edit.place_caret(11);
    edit.sync_external("hi".to_owned());
    assert_eq!(edit.as_str(), "hi");
    assert_eq!(edit.caret(), 2, "caret clamps to the new (shorter) length");
}

#[test]
fn sync_external_snaps_caret_off_new_boundary() {
    let mut edit = TextEdit::new_at("world".to_owned(), usize::MAX);
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
    let mut edit = TextEdit::new_at("hello".to_owned(), usize::MAX);
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
fn rtl_arrow_keys_move_visually() {
    let mut edit = TextEdit::new_at("אבג".to_owned(), usize::MAX);
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
    let mut edit = TextEdit::new_at("abc".to_owned(), usize::MAX);
    edit.place_caret(1);
    edit.move_right();
    assert_eq!(edit.caret(), 2);
    edit.move_left();
    assert_eq!(edit.caret(), 1);
}

#[test]
fn word_motion_and_deletion_follow_word_boundaries() {
    let mut edit = TextEdit::new_at("fix the  frame_loop, then ship".to_owned(), usize::MAX);
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
    let mut edit = TextEdit::new_at("first line\nsecond line\nthird".to_owned(), usize::MAX);
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
