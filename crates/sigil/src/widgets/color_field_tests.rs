use crate::style::Color;

#[test]
fn hex_roundtrip() {
    let c: Color = "#0af".parse().expect("#0af is valid hex");
    assert_eq!(
        c,
        Color {
            r: 0,
            g: 170,
            b: 255,
            a: 255
        }
    );
    assert_eq!(c.to_hex(), "#00aaff");
    assert!("red".parse::<Color>().is_err());
}
