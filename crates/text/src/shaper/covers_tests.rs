#![allow(clippy::unwrap_used)]

use super::Font;

const FIRA_REGULAR: &[u8] = include_bytes!("../../assets/fonts/FiraSans-Regular.ttf");

fn fira_regular() -> Font {
    Font::from_bytes(FIRA_REGULAR.to_vec(), 0).expect("FiraSans-Regular fixture must parse")
}

#[test]
fn covers_treats_variation_selector_16_as_transparent() {
    let font = fira_regular();
    assert!(
        font.covers("\u{FE0F}"),
        "a lone VS16 must not fail coverage — it's a default-ignorable control",
    );
    assert!(
        font.covers("a\u{FE0F}b"),
        "VS16 between two covered Latin letters must not fail coverage",
    );
}

#[test]
fn covers_treats_combining_grapheme_joiner_as_transparent() {
    let font = fira_regular();
    assert!(font.covers("\u{034F}"), "a lone CGJ must not fail coverage");
    assert!(
        font.covers("a\u{034F}b"),
        "CGJ between two covered Latin letters must not fail coverage",
    );
}

#[test]
fn covers_treats_word_joiner_as_transparent() {
    let font = fira_regular();
    assert!(
        font.covers("\u{2060}"),
        "a lone WORD JOINER must not fail coverage"
    );
    assert!(
        font.covers("a\u{2060}b"),
        "WORD JOINER between two covered Latin letters must not fail coverage",
    );
}

#[test]
fn covers_treats_supplementary_variation_selector_as_transparent() {
    let font = fira_regular();
    assert!(
        font.covers("\u{E0100}"),
        "a lone supplementary variation selector must not fail coverage",
    );
    assert!(
        font.covers("a\u{E0100}b"),
        "a supplementary VS between two covered Latin letters must not fail coverage",
    );
}

#[test]
fn covers_still_rejects_a_genuinely_uncovered_base_character() {
    let font = fira_regular();
    assert!(
        !font.covers("\u{5B57}"),
        "a genuinely uncovered non-ignorable codepoint must still fail coverage",
    );
    assert!(
        !font.covers("\u{5B57}\u{FE0F}"),
        "an uncovered base character must still fail coverage even when \
         followed by a default-ignorable control",
    );
}
