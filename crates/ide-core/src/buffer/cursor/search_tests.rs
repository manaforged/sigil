use super::*;

#[test]
fn add_next_match_finds_three_occurrences() {
    let rope = Rope::from_str("foo bar foo baz foo");
    let mut set = CursorSet::new(Cursor::at(0));
    assert!(set.add_next_match(&rope, "foo"));
    assert!(set.add_next_match(&rope, "foo"));
    assert!(set.add_next_match(&rope, "foo"));
    let spans: Vec<_> = set.cursors().iter().map(Cursor::span).collect();
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
    let starts: Vec<usize> = set.cursors().iter().map(|c| c.span().start).collect();
    assert!(starts.contains(&0));
    assert!(starts.contains(&10));
}

#[test]
fn add_next_match_skips_when_pattern_inside_existing_selection() {
    let rope = Rope::from_str("foo bar foo baz");
    let mut set = CursorSet::new(Cursor::selection(0, 3));
    assert!(set.add_next_match(&rope, "foo"));
    assert_eq!(set.cursors().len(), 2);
    assert_eq!(set.cursors()[1].span(), 8..11);
}
