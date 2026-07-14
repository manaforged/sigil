//! Unit tests for [`super::buffer::TextEdit`], split out of
//! `buffer.rs` to keep the production file under the 400-line cap
//! (rust-hygiene structural rule).

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
    // "café" — the 'é' is 2 bytes (3..5), so byte 4 is mid-codepoint.
    let mut edit = TextEdit::new("café".to_owned());
    edit.place_caret(999);
    assert_eq!(edit.caret(), edit.len(), "past-end clamps to length");
    edit.place_caret(4);
    assert_eq!(edit.caret(), 3, "mid-codepoint snaps back to a boundary");
}

#[test]
fn extend_to_anchors_at_starting_caret_then_grows() {
    let mut edit = TextEdit::new("hello world".to_owned());
    edit.place_caret(2); // caret at 2, no selection
    edit.extend_to(6); // drag right to byte 6 (exclusive)
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
    edit.extend_to(1); // drag back to the origin
    assert_eq!(edit.selection(), None, "a zero-width drag collapses");
    assert_eq!(edit.caret(), 1);
}

/// W5-IME-TE HIGH-2 regression coverage at the buffer layer: an
/// external write shorter than the current caret must clamp, not
/// panic or leave the caret pointing past the new buffer's end.
#[test]
fn sync_external_clamps_caret_past_new_end() {
    let mut edit = TextEdit::new("hello world".to_owned());
    edit.place_caret(11); // end of "hello world"
    edit.sync_external("hi".to_owned());
    assert_eq!(edit.as_str(), "hi");
    assert_eq!(edit.caret(), 2, "caret clamps to the new (shorter) length");
}

/// W5-IME-TE HIGH-1/HIGH-2: a caret valid in the old buffer can land
/// mid-codepoint in the new one — must snap back to a real boundary,
/// never panic.
#[test]
fn sync_external_snaps_caret_off_new_boundary() {
    let mut edit = TextEdit::new("world".to_owned());
    edit.place_caret(4); // valid boundary in "world" (between 'l' and 'd')
    // "café" — byte 4 sits mid-codepoint of the 2-byte 'é' (bytes 3..5).
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
