use super::super::Cursor;
use super::*;

fn text(rope: &Rope) -> String {
    rope.to_string_lossy()
}

#[test]
fn three_cursors_parallel_insert_remap_correct() {
    let rope = Rope::from_str("abcdefghij");
    let mut set = CursorSet::new(Cursor::at(2));
    set.add(Cursor::at(5));
    set.add(Cursor::at(8));
    let new_rope = set.edit(&rope, |c, _| {
        Some(Edit::Insert {
            at: c.head,
            text: "X".into(),
        })
    });
    assert_eq!(text(&new_rope), "abXcdeXfghXij");
    let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
    assert_eq!(heads, vec![3, 7, 11]);
}

#[test]
fn parallel_delete_remap_correct() {
    let rope = Rope::from_str("abcdefghij");
    let mut set = CursorSet::new(Cursor::selection(1, 3));
    set.add(Cursor::selection(6, 8));
    let new_rope = set.edit(&rope, |c, _| {
        let s = c.span();
        Some(Edit::Delete {
            from: s.start,
            to: s.end,
        })
    });
    assert_eq!(text(&new_rope), "adefij");
    let cursors = set.cursors();
    assert_eq!(cursors[0].head, 1);
    assert_eq!(cursors[0].anchor, 1);
    assert_eq!(cursors[1].head, 4);
}

#[test]
fn overlapping_delete_produces_one_edit() {
    let rope = Rope::from_str("abcdefghij");
    let mut set = CursorSet::new(Cursor::selection(2, 6));
    set.add(Cursor::selection(4, 8));
    let new_rope = set.edit(&rope, |c, _| {
        let s = c.span();
        Some(Edit::Delete {
            from: s.start,
            to: s.end,
        })
    });
    assert_eq!(text(&new_rope), "abij");
    assert_eq!(set.cursors().len(), 1);
}

#[test]
fn replace_remaps_trailing_cursors() {
    let rope = Rope::from_str("hello world today");
    let mut set = CursorSet::new(Cursor::at(0));
    set.add(Cursor::selection(6, 11));
    set.add(Cursor::at(17));
    let new_rope = set.edit(&rope, |c, _| {
        if c.span().start == 6 {
            Some(Edit::Replace {
                from: 6,
                to: 11,
                text: "earth!".into(),
            })
        } else {
            None
        }
    });
    assert_eq!(text(&new_rope), "hello earth! today");
    let cursors = set.cursors();
    assert_eq!(cursors[0].head, 0);
    assert_eq!(cursors[1].head, 12);
    assert_eq!(cursors[2].head, 18);
}

#[test]
fn noop_edit_returns_clone_of_original() {
    let rope = Rope::from_str("hello world");
    let mut set = CursorSet::new(Cursor::at(5));
    set.add(Cursor::at(11));
    let new_rope = set.edit(&rope, |_, _| None);
    assert_eq!(text(&new_rope), "hello world");
    assert_eq!(set.cursors().len(), 2);
}

#[test]
fn insert_at_buffer_end_works() {
    let rope = Rope::from_str("abc");
    let mut set = CursorSet::new(Cursor::at(3));
    let new_rope = set.edit(&rope, |c, _| {
        Some(Edit::Insert {
            at: c.head,
            text: "!".into(),
        })
    });
    assert_eq!(text(&new_rope), "abc!");
    assert_eq!(set.cursors()[0].head, 4);
}
