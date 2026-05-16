#![allow(missing_docs)]

use sigil as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use taffy as _;

#[path = "../src/widgets/text_edit.rs"]
mod text_edit;

use text_edit::TextEdit;

#[test]
fn insert_ascii_advances_caret() {
    let mut edit = TextEdit::new(String::new());

    edit.insert_str("abc");

    assert_eq!(edit.as_str(), "abc");
    assert_eq!(edit.caret(), 3);
    assert_eq!(edit.len(), 3);
    assert!(!edit.is_empty());
}

#[test]
fn insert_multibyte_emoji_advances_by_grapheme_bytes() {
    let mut edit = TextEdit::new(String::new());

    edit.insert_str("🙂");

    assert_eq!(edit.as_str(), "🙂");
    assert_eq!(edit.caret(), "🙂".len());
}

#[test]
fn backspace_at_start_is_noop() {
    let mut edit = TextEdit::new(String::from("abc"));
    edit.move_home();

    edit.backspace();

    assert_eq!(edit.as_str(), "abc");
    assert_eq!(edit.caret(), 0);
}

#[test]
fn backspace_before_multibyte_boundary_removes_full_codepoint() {
    let mut edit = TextEdit::new(String::from("a🙂b"));
    edit.move_left();

    edit.backspace();

    assert_eq!(edit.as_str(), "ab");
    assert_eq!(edit.caret(), 1);
}

#[test]
fn delete_forward_at_end_is_noop() {
    let mut edit = TextEdit::new(String::from("abc"));

    edit.delete_forward();

    assert_eq!(edit.as_str(), "abc");
    assert_eq!(edit.caret(), 3);
}

#[test]
fn caret_never_mid_char_after_any_operation() {
    let mut edit = TextEdit::new(String::from("a🙂é"));
    assert_on_boundary(&edit);

    edit.move_left();
    assert_on_boundary(&edit);
    edit.move_left();
    assert_on_boundary(&edit);
    edit.insert_str("ø");
    assert_on_boundary(&edit);
    edit.backspace();
    assert_on_boundary(&edit);
    edit.delete_forward();
    assert_on_boundary(&edit);
    edit.move_right();
    assert_on_boundary(&edit);
}

#[test]
fn home_and_end_jump_to_bounds() {
    let mut edit = TextEdit::new(String::from("a🙂b"));

    edit.move_home();
    assert_eq!(edit.caret(), 0);

    edit.move_end();
    assert_eq!(edit.caret(), edit.len());
}

fn assert_on_boundary(edit: &TextEdit) {
    assert!(edit.caret() <= edit.len());
    assert!(edit.as_str().is_char_boundary(edit.caret()));
}

#[test]
fn shift_right_extends_selection() {
    let mut edit = TextEdit::new(String::from("abc"));
    edit.move_home();

    edit.shift_right();
    edit.shift_right();

    assert_eq!(edit.selected_text(), Some("ab"));
    assert_eq!(edit.caret(), 2);
}

#[test]
fn shift_left_extends_selection_backward() {
    let mut edit = TextEdit::new(String::from("abc"));

    edit.shift_left();
    edit.shift_left();

    assert_eq!(edit.selected_text(), Some("bc"));
    assert_eq!(edit.caret(), 1);
}

#[test]
fn shift_home_selects_to_start() {
    let mut edit = TextEdit::new(String::from("hello"));

    edit.shift_home();

    assert_eq!(edit.selected_text(), Some("hello"));
    assert_eq!(edit.caret(), 0);
}

#[test]
fn shift_end_selects_to_end() {
    let mut edit = TextEdit::new(String::from("hello"));
    edit.move_home();

    edit.shift_end();

    assert_eq!(edit.selected_text(), Some("hello"));
    assert_eq!(edit.caret(), 5);
}

#[test]
fn insert_replaces_selection() {
    let mut edit = TextEdit::new(String::from("hello world"));
    edit.move_home();
    edit.shift_right();
    edit.shift_right();
    edit.shift_right();
    edit.shift_right();
    edit.shift_right();

    edit.insert_str("hi");

    assert_eq!(edit.as_str(), "hi world");
    assert!(edit.selected_text().is_none());
}

#[test]
fn backspace_deletes_selection() {
    let mut edit = TextEdit::new(String::from("abcd"));
    edit.move_home();
    edit.shift_right();
    edit.shift_right();

    edit.backspace();

    assert_eq!(edit.as_str(), "cd");
    assert_eq!(edit.caret(), 0);
}

#[test]
fn select_all_covers_buffer() {
    let mut edit = TextEdit::new(String::from("hello"));

    edit.select_all();

    assert_eq!(edit.selected_text(), Some("hello"));
}

#[test]
fn move_collapses_selection() {
    let mut edit = TextEdit::new(String::from("abc"));
    edit.move_home();
    edit.shift_right();
    edit.shift_right();
    assert!(edit.has_selection());

    edit.move_right();

    assert!(!edit.has_selection());
}
