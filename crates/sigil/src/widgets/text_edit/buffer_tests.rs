#![allow(clippy::shadow_unrelated)]

use super::TextEdit;

#[test]
fn editing_keeps_graphemes_whole() {
    for grapheme in ["👍🏽", "e\u{301}", "👩‍💻", "🇳🇿"] {
        let text = format!("A{grapheme}B");
        let mut edit = TextEdit::new_at(text.clone(), usize::MAX);
        edit.move_by(false, false);
        edit.backspace();
        assert_eq!(edit.as_str(), "AB");
        assert!(edit.undo());
        assert_eq!(edit.as_str(), text);
        edit.move_by(false, false);
        assert_eq!(edit.caret(), 1);
        edit.delete_forward();
        assert_eq!(edit.as_str(), "AB");
        let mut selection = TextEdit::new_at(text, usize::MAX);
        selection.place_caret(1 + grapheme.len() - 1);
        assert_eq!(selection.caret(), 1);
        selection.move_by(true, true);
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
    assert!(edit.as_str().is_empty());
    let mut insertion = TextEdit::new_at("\u{301}".to_string(), 0);
    insertion.insert_str("e");
    assert_eq!(insertion.caret(), "e\u{301}".len());
    insertion.backspace();
    assert!(insertion.as_str().is_empty());
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

fn at(text: &str, caret: usize) -> TextEdit {
    TextEdit::new_at(text.to_owned(), caret)
}

#[test]
fn place_caret_collapses_selection_and_clamps_to_a_boundary() {
    let mut edit = at("hello world", usize::MAX);
    edit.select_all();
    assert!(edit.selection().is_some());
    edit.place_caret(3);
    assert_eq!(edit.caret(), 3);
    assert_eq!(edit.selection(), None, "click collapses the selection");
    let mut edit = at("café", usize::MAX);
    edit.place_caret(999);
    assert_eq!(edit.caret(), edit.len(), "past-end clamps to length");
    edit.place_caret(4);
    assert_eq!(edit.caret(), 3, "mid-codepoint snaps back to a boundary");
}

#[test]
fn extend_to_anchors_at_the_drag_origin_and_collapses_on_it() {
    let mut edit = at("hello world", 2);
    edit.extend_to(6);
    assert_eq!(edit.caret(), 6);
    assert_eq!(edit.selection(), Some(2), "anchor stays at the drag origin");
    assert_eq!(edit.selected_text(), Some("llo "));
    edit.extend_to(2);
    assert_eq!(edit.selection(), None, "a zero-width drag collapses");
    assert_eq!(edit.caret(), 2);
}

#[test]
fn sync_external_clamps_the_caret_and_ignores_a_matching_value() {
    let mut edit = at("hello world", 11);
    edit.sync_external("hi".to_owned());
    assert_eq!(edit.as_str(), "hi");
    assert_eq!(edit.caret(), 2, "caret clamps to the new (shorter) length");
    let mut edit = at("world", 4);
    edit.sync_external("café".to_owned());
    assert_eq!(
        edit.caret(),
        3,
        "mid-codepoint offset snaps back to a boundary"
    );
    let mut edit = at("hello", 3);
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
fn arrow_keys_move_visually_on_rtl_and_logically_on_ltr() {
    let mut edit = at("אבג", usize::MAX);
    assert_eq!(edit.caret(), 6, "caret starts at logical end");
    for expected in [4, 2, 0, 0] {
        edit.move_by(true, false);
        assert_eq!(
            edit.caret(),
            expected,
            "visual-right = logical back, stopping at the edge"
        );
    }
    edit.move_by(false, false);
    assert_eq!(edit.caret(), 2, "visual-left = logical forward");
    let mut edit = at("abc", 1);
    edit.move_by(true, false);
    assert_eq!(edit.caret(), 2);
    edit.move_by(false, false);
    assert_eq!(edit.caret(), 1);
    let text = "x\nאבג";
    let mut edit = at(text, text.len());
    edit.move_by(true, false);
    assert_eq!(edit.caret(), text.len() - "ג".len());
}

#[test]
fn word_motion_and_deletion_follow_word_boundaries() {
    let mut edit = at("fix the  frame_loop, then ship", usize::MAX);
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
    let mut edit = at("first line\nsecond line\nthird", usize::MAX);
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
