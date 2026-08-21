use super::{parse_hex, to_hex};
use sigil_style::Rgba8;

#[test]
fn hex_roundtrip() {
    let c = parse_hex("#0af").expect("#0af is valid hex");
    assert_eq!(
        c,
        Rgba8 {
            r: 0,
            g: 170,
            b: 255,
            a: 255
        }
    );
    assert_eq!(to_hex(c), "#00aaff");
    assert!(parse_hex("red").is_none());
}
