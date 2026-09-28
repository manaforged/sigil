use super::*;

#[test]
fn parses_base_and_hex_tokens() {
    let theme = parse("base = \"dark\"\n[tokens]\naccent = \"#7c3aed\"\nsoft = \"#11223344\"\n")
        .expect("parses");
    assert_eq!(
        theme.tokens.overrides.value("accent"),
        Some(ThemeValue::Color(Rgba8 {
            r: 0x7c,
            g: 0x3a,
            b: 0xed,
            a: 255,
        })),
    );
    assert_eq!(
        theme.tokens.overrides.value("soft"),
        Some(ThemeValue::Color(Rgba8 {
            r: 0x11,
            g: 0x22,
            b: 0x33,
            a: 0x44,
        })),
    );
}

#[test]
fn light_is_the_default_base() {
    let theme = parse("[tokens]\naccent = \"0x102030\"").expect("parses");
    assert_eq!(
        theme.tokens.overrides.value("accent"),
        Some(ThemeValue::Color(Rgba8 {
            r: 0x10,
            g: 0x20,
            b: 0x30,
            a: 255,
        })),
    );
}

#[test]
fn rejects_bad_base_bad_hex_and_non_toml() {
    assert!(parse("base = \"neon\"\n[tokens]\na = \"#ffffff\"").is_none());
    assert!(parse("[tokens]\na = \"#12\"").is_none());
    assert!(parse("not toml at all [[[").is_none());
}
