use super::*;

#[test]
fn tier_ordering() {
    const { assert!(EXACT > EXACT_ICASE) };
    const { assert!(EXACT_ICASE > PREFIX) };
    const { assert!(PREFIX > PREFIX_ICASE) };
}

#[test]
fn path_segment_outranks_word_boundary() {
    const { assert!(PATH_SEGMENT_BONUS > WORD_BOUNDARY_BONUS) };
}

#[test]
fn camel_detected() {
    assert!(is_camel_boundary('O', Some(b'c')));
    assert!(!is_camel_boundary('o', Some(b'c')));
    assert!(!is_camel_boundary('O', Some(b'C')));
}

#[test]
fn utf8_len_handles_ascii_and_multibyte() {
    assert_eq!(utf8_len(b"a", 0), 1);
    let s = "é";
    assert_eq!(utf8_len(s.as_bytes(), 0), 2);
    assert_eq!(utf8_len(b"a", 9), 1);
}
