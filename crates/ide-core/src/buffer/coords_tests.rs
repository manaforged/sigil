use super::*;

#[test]
fn counts_match_str_chars_lines_utf16() {
    let s = "hello\nworld\n🦀abc";
    let b = s.as_bytes();
    assert_eq!(count_chars(b), s.chars().count());
    assert_eq!(count_lines(b), s.bytes().filter(|&c| c == b'\n').count());
    assert_eq!(count_utf16(b), s.encode_utf16().count());
}

#[test]
fn byte_char_roundtrips_in_leaf() {
    let s = "α🦀β\nγ";
    let b = s.as_bytes();
    for (ci, (bi, _)) in s.char_indices().enumerate() {
        assert_eq!(byte_to_char_in_leaf(b, bi), ci);
        assert_eq!(char_to_byte_in_leaf(b, ci), bi);
    }
}

#[test]
fn line_starts_in_leaf() {
    let s = "a\nbb\n\nccc";
    let b = s.as_bytes();
    assert_eq!(line_start_in_leaf(b, 0), 0);
    assert_eq!(line_start_in_leaf(b, 1), 2);
    assert_eq!(line_start_in_leaf(b, 2), 5);
    assert_eq!(line_start_in_leaf(b, 3), 6);
    assert_eq!(line_start_in_leaf(b, 99), b.len());
}

#[test]
fn utf16_units_match_encode_utf16() {
    let s = "🦀a🦀b";
    let b = s.as_bytes();
    let total = count_utf16(b);
    assert_eq!(total, s.encode_utf16().count());
    let pos = byte_to_utf16_in_leaf(b, 4);
    assert!(pos > 0);
}
