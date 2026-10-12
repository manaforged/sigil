use super::*;

fn color(theme: &Theme, token: &str) -> Option<ThemeValue> {
    theme.tokens.overrides.value(token)
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> ThemeValue {
    ThemeValue::Color(Rgba8 { r, g, b, a })
}

#[test]
fn parses_base_and_hex_tokens() {
    let theme = parse("base = \"dark\"\n[tokens]\naccent = \"#7c3aed\"\nsoft = \"#11223344\"\n")
        .expect("parses");
    assert_eq!(color(&theme, "accent"), Some(rgba(0x7c, 0x3a, 0xed, 255)));
    assert_eq!(color(&theme, "soft"), Some(rgba(0x11, 0x22, 0x33, 0x44)));
}

#[test]
fn light_is_the_default_base() {
    let theme = parse("[tokens]\naccent = \"0x102030\"").expect("parses");
    assert_eq!(color(&theme, "accent"), Some(rgba(0x10, 0x20, 0x30, 255)));
}

#[test]
fn rejects_bad_base_bad_hex_and_non_toml() {
    assert!(parse("base = \"neon\"\n[tokens]\na = \"#ffffff\"").is_none());
    assert!(parse("[tokens]\na = \"#12\"").is_none());
    assert!(parse("not toml at all [[[").is_none());
}
