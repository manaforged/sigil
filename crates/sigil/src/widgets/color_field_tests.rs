use crate::style::Rgba8;

#[test]
fn hex_roundtrip() {
    let c: Rgba8 = "#0af".parse().expect("#0af is valid hex");
    assert_eq!(
        c,
        Rgba8 {
            r: 0,
            g: 170,
            b: 255,
            a: 255
        }
    );
    assert_eq!(c.to_hex(), "#00aaff");
    assert!("red".parse::<Rgba8>().is_err());
}
