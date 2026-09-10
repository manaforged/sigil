use super::*;

fn text(rope: &Rope) -> String {
    rope.to_string_lossy()
}

#[test]
fn single_cursor_insert() {
    let rope = Rope::from_str("hello world");
    let mut set = CursorSet::new(Cursor::at(5));
    let new_rope = set.edit(&rope, |c, _| {
        Some(Edit::Insert {
            at: c.head,
            text: ",".into(),
        })
    });
    assert_eq!(text(&new_rope), "hello, world");
    assert_eq!(set.cursors()[0].head, 6);
}

#[test]
fn single_cursor_delete() {
    let rope = Rope::from_str("hello world");
    let mut set = CursorSet::new(Cursor::selection(0, 6));
    let new_rope = set.edit(&rope, |c, _| {
        let s = c.span();
        Some(Edit::Delete {
            from: s.start,
            to: s.end,
        })
    });
    assert_eq!(text(&new_rope), "world");
    assert_eq!(set.cursors()[0].head, 0);
    assert!(set.cursors()[0].is_caret());
}

#[test]
fn add_preserves_sort_order() {
    let mut set = CursorSet::new(Cursor::at(10));
    set.add(Cursor::at(2));
    set.add(Cursor::at(7));
    set.add(Cursor::at(0));
    set.add(Cursor::at(15));
    let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
    assert_eq!(heads, vec![0, 2, 7, 10, 15]);
}

#[test]
fn overlapping_selections_coalesce() {
    let mut set = CursorSet::new(Cursor::selection(5, 10));
    set.add(Cursor::selection(8, 12));
    set.coalesce();
    assert_eq!(set.cursors().len(), 1);
    assert_eq!(set.cursors()[0].span(), 5..12);
}

#[test]
fn touching_selections_coalesce() {
    let mut set = CursorSet::new(Cursor::selection(0, 5));
    set.add(Cursor::selection(5, 10));
    set.coalesce();
    assert_eq!(set.cursors().len(), 1);
    assert_eq!(set.cursors()[0].span(), 0..10);
}

#[test]
fn contained_selection_coalesce() {
    let mut set = CursorSet::new(Cursor::selection(2, 10));
    set.add(Cursor::selection(4, 6));
    set.coalesce();
    assert_eq!(set.cursors().len(), 1);
    assert_eq!(set.cursors()[0].span(), 2..10);
}

#[test]
fn coalesce_preserves_forward_direction() {
    let mut set = CursorSet::new(Cursor::selection(0, 5));
    set.add(Cursor::selection(8, 4));
    set.coalesce();
    assert_eq!(set.cursors().len(), 1);
    let c = set.cursors()[0];
    assert_eq!(c.span(), 0..8);
    assert!(
        c.is_forward(),
        "merged should be forward when any input was"
    );
}

#[test]
fn extend_to_preserves_anchor() {
    let mut set = CursorSet::new(Cursor::at(5));
    set.extend_to(10);
    let c = set.cursors()[0];
    assert_eq!(c.anchor, 5);
    assert_eq!(c.head, 10);
}

#[test]
fn set_primary_replaces() {
    let mut set = CursorSet::new(Cursor::at(0));
    set.add_at(5);
    set.add_at(10);
    set.set_primary(Cursor::at(42));
    assert_eq!(set.cursors().len(), 1);
    assert_eq!(set.cursors()[0].head, 42);
}

#[test]
fn primary_returns_first() {
    let mut set = CursorSet::new(Cursor::at(10));
    set.add(Cursor::at(2));
    let p = set.primary().expect("at least one cursor");
    assert_eq!(p.head, 2);
}

#[test]
fn default_is_caret_at_zero() {
    let set = CursorSet::default();
    assert_eq!(set.len(), 1);
    assert_eq!(set.cursors()[0], Cursor::at(0));
}
