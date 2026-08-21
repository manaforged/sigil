#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc
)]

use super::*;

#[test]
fn empty_rope_construction() {
    let r = Rope::new();
    assert_eq!(r.len_bytes(), 0);
    assert_eq!(r.len_chars(), 0);
    assert_eq!(r.len_lines(), 0);
    assert!(r.is_empty());
}

#[test]
fn from_str_counts() {
    let r = Rope::from_str("hello\nworld\n");
    assert_eq!(r.len_bytes(), 12);
    assert_eq!(r.len_chars(), 12);
    assert_eq!(r.len_lines(), 2);
}

#[test]
fn single_byte_insert_remove() {
    let mut r = Rope::new();
    r.insert(0, "a");
    assert_eq!(r.len_bytes(), 1);
    assert_eq!(r.to_string_lossy(), "a");
    r.remove(0..1);
    assert_eq!(r.len_bytes(), 0);
    assert!(r.is_empty());
}

#[test]
fn insert_at_boundaries() {
    let mut r = Rope::from_str("world");
    r.insert(0, "hello ");
    assert_eq!(r.to_string_lossy(), "hello world");
    r.insert(r.len_bytes(), "!");
    assert_eq!(r.to_string_lossy(), "hello world!");
    r.insert(6, "beautiful ");
    assert_eq!(r.to_string_lossy(), "hello beautiful world!");
}

#[test]
fn utf8_boundary_insert() {
    // "café" — 'é' is 2 bytes (U+00E9).
    let mut r = Rope::from_str("caf");
    r.insert(3, "é");
    assert_eq!(r.to_string_lossy(), "café");
    assert_eq!(r.len_chars(), 4);
    assert_eq!(r.len_bytes(), 5);
}

#[test]
fn cross_leaf_insert() {
    // Build a rope bigger than one leaf, then insert in the middle.
    let base: String = "a".repeat(LEAF_TARGET + 10);
    let mut r = Rope::from_str(&base);
    let mid = LEAF_TARGET / 2;
    r.insert(mid, "X");
    let s = r.to_string_lossy();
    assert_eq!(s.len(), base.len() + 1);
    assert_eq!(s.as_bytes().get(mid), Some(&b'X'));
}

#[test]
fn cross_leaf_remove() {
    let base: String = "abcdefgh".repeat(200); // 1600 bytes, 3+ leaves
    let mut r = Rope::from_str(&base);
    r.remove(500..510);
    let s = r.to_string_lossy();
    assert_eq!(s.len(), base.len() - 10);
    // Verify the content around the gap is correct.
    let mut expected = base;
    expected.replace_range(500..510, "");
    assert_eq!(s, expected);
}

#[test]
fn to_string_roundtrip_large() {
    let source: String = "hello world\n".repeat(1000);
    let r = Rope::from_str(&source);
    assert_eq!(r.to_string_lossy(), source);
}

#[test]
fn slice_correctness() {
    let r = Rope::from_str("hello world");
    let sl = r.slice(6..11);
    assert_eq!(sl.to_string(), "world");
}

#[test]
fn line_access() {
    let r = Rope::from_str("foo\nbar\nbaz");
    assert_eq!(r.line(0).to_string(), "foo\n");
    assert_eq!(r.line(1).to_string(), "bar\n");
    assert_eq!(r.line(2).to_string(), "baz");
}

#[test]
fn snapshot_is_independent() {
    let r = Rope::from_str("hello");
    let snap = r.snapshot();
    let mut r2 = r;
    r2.insert(5, " world");
    assert_eq!(snap.to_string_lossy(), "hello");
    assert_eq!(r2.to_string_lossy(), "hello world");
}

#[test]
fn byte_to_char_conversion() {
    let r = Rope::from_str("🦀abc");
    // '🦀' is 4 bytes; byte 4 should be char index 1.
    assert_eq!(r.byte_to_char(ByteIdx(4)), CharIdx(1));
    assert_eq!(r.char_to_byte(CharIdx(1)), ByteIdx(4));
}

#[test]
fn byte_to_linecol_conversion() {
    let r = Rope::from_str("hello\nworld");
    let lc = r.byte_to_linecol(ByteIdx(7));
    assert_eq!(lc.line, 1);
    assert_eq!(lc.col, 1);
}

#[test]
fn write_to_roundtrip() {
    let source = "hello world\n";
    let r = Rope::from_str(source);
    let mut out: Vec<u8> = Vec::new();
    r.write_to(&mut out).unwrap();
    assert_eq!(out, source.as_bytes());
}

#[test]
fn large_insert() {
    let big = "x".repeat(1_024 * 1_024); // 1 MB
    let r = Rope::from_str(&big);
    assert_eq!(r.len_bytes(), big.len());
    assert_eq!(r.to_string_lossy(), big);
}
