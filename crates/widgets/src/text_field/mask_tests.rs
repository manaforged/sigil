use super::{mask_byte, mask_chars, unmask_byte};

#[test]
fn mask_chars_one_star_per_scalar() {
    assert_eq!(mask_chars(""), "");
    assert_eq!(mask_chars("ab"), "**");
    assert_eq!(mask_chars("é"), "*");
    assert_eq!(mask_chars("a💡"), "**");
}

#[test]
fn mask_byte_counts_scalars_not_utf8() {
    assert_eq!(mask_byte("ab", 0), 0);
    assert_eq!(mask_byte("ab", 1), 1);
    assert_eq!(mask_byte("ab", 2), 2);
    assert_eq!(mask_byte("é", "é".len()), 1);
    assert_eq!(mask_byte("a💡", 1), 1);
}

#[test]
fn unmask_byte_roundtrips_ascii_and_multibyte() {
    let value = "a💡b";
    for byte in [0, 1, value.len()] {
        let masked = mask_byte(value, byte);
        assert_eq!(unmask_byte(value, masked), byte);
    }
    let mid = 1 + "💡".len();
    assert_eq!(unmask_byte(value, mask_byte(value, mid)), mid);
}
