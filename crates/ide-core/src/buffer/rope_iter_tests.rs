#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;

#[test]
fn bytes_iter_correct() {
    let data = b"hello";
    let v: Vec<u8> = RopeBytesIter::new(data).collect();
    assert_eq!(v, b"hello");
}

#[test]
fn chars_iter_correct() {
    let s = "abc🦀";
    let v: Vec<char> = RopeCharsIter::new(s).collect();
    assert_eq!(v, s.chars().collect::<Vec<_>>());
}

#[test]
fn lines_iter_correct() {
    let s = "a\nbb\ncc";
    let v: Vec<&str> = RopeLinesIter::new(s).collect();
    assert_eq!(v, vec!["a\n", "bb\n", "cc"]);
}

#[test]
fn lines_iter_empty() {
    assert!(RopeLinesIter::new("").next().is_none());
}

#[test]
fn lines_iter_trailing_newline() {
    let s = "a\nb\n";
    let v: Vec<&str> = RopeLinesIter::new(s).collect();
    assert_eq!(v, vec!["a\n", "b\n"]);
}
