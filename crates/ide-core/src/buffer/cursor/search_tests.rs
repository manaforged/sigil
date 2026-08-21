#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;

#[test]
fn add_next_match_finds_three_occurrences() {
    let rope = Rope::from_str("foo bar foo baz foo");
    let mut set = CursorSet::new(Cursor::at(0));
    assert!(set.add_next_match(&rope, "foo"));
    assert!(set.add_next_match(&rope, "foo"));
    assert!(set.add_next_match(&rope, "foo"));
    let spans: Vec<_> = set.cursors().iter().map(Cursor::span).collect();
    // Original caret at 0 + three "foo" matches at 0, 8, 16.
    // The first match starts at 0; it coalesces with the caret-at-0.
    // Then matches at 8 and 16 add two more selections.
    // Total: original-coalesced + 2 = at least 3 cursors covering 0, 8, 16.
    let starts: Vec<usize> = spans.iter().map(|r| r.start).collect();
    assert!(
        starts.contains(&0) && starts.contains(&8) && starts.contains(&16),
        "starts = {starts:?}",
    );
}

#[test]
fn add_next_match_no_more_returns_false() {
    let rope = Rope::from_str("foo bar");
    let mut set = CursorSet::new(Cursor::at(0));
    assert!(set.add_next_match(&rope, "foo"));
    // No second "foo" exists.
    assert!(!set.add_next_match(&rope, "foo"));
}

#[test]
fn add_next_match_empty_pattern_is_noop() {
    let rope = Rope::from_str("hello");
    let mut set = CursorSet::new(Cursor::at(0));
    assert!(!set.add_next_match(&rope, ""));
    assert_eq!(set.cursors().len(), 1);
}

#[test]
fn add_next_match_finds_unicode_pattern() {
    let rope = Rope::from_str("café and café again");
    let mut set = CursorSet::new(Cursor::at(0));
    assert!(set.add_next_match(&rope, "café"));
    assert!(set.add_next_match(&rope, "café"));
    // "café" is 5 bytes (é is 2 bytes). First match at 0, second at 10.
    let starts: Vec<usize> = set.cursors().iter().map(|c| c.span().start).collect();
    assert!(starts.contains(&0));
    assert!(starts.contains(&10));
}

#[test]
fn add_next_match_skips_when_pattern_inside_existing_selection() {
    // First cursor already selects "foo"; the SECOND search starts AFTER
    // its end, so the next match (8..11) gets added cleanly.
    let rope = Rope::from_str("foo bar foo baz");
    let mut set = CursorSet::new(Cursor::selection(0, 3));
    assert!(set.add_next_match(&rope, "foo"));
    assert_eq!(set.cursors().len(), 2);
    assert_eq!(set.cursors()[1].span(), 8..11);
}
