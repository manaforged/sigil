//! Unit tests for `Font::covers`'s default-ignorable handling (adversarial
//! review, W4 fixes 2 + A) — split out of `shaper/tests.rs`
//! to keep that file under the 400-line cap.

#![allow(clippy::unwrap_used)]

use super::Font;

const FIRA_REGULAR: &[u8] = include_bytes!("../../assets/fonts/FiraSans-Regular.ttf");

fn fira_regular() -> Font {
    Font::from_bytes(FIRA_REGULAR.to_vec(), 0).expect("FiraSans-Regular fixture must parse")
}

#[test]
fn covers_treats_variation_selector_16_as_transparent() {
    // FiraSans's cmap has no VS16 (U+FE0F) entry (confirmed via a
    // charmap probe) — it's a GSUB shaping control, never drawn on its
    // own. Requiring a cmap glyph for it wrongly rejected an otherwise-
    // covering face.
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
    // U+034F COMBINING GRAPHEME JOINER — no cmap entry in FiraSans
    // (confirmed via a charmap probe), a Unicode `Default_Ignorable_
    // Code_Point` the range-table classifier must recognise beyond the
    // 4 hand-picked chars the first fix hardcoded.
    let font = fira_regular();
    assert!(font.covers("\u{034F}"), "a lone CGJ must not fail coverage");
    assert!(
        font.covers("a\u{034F}b"),
        "CGJ between two covered Latin letters must not fail coverage",
    );
}

#[test]
fn covers_treats_word_joiner_as_transparent() {
    // U+2060 WORD JOINER — no cmap entry in FiraSans.
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
    // U+E0100 VARIATION SELECTOR-17 (the first of the supplementary
    // block, U+E0100..=U+E01EF) — no cmap entry in FiraSans, and NOT
    // covered by the first fix's 4-char hardcoded set at all.
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
    // Control: the ignorable-skip must not rubber-stamp `covers` — a
    // real uncovered codepoint (per `notdef_for_unknown_codepoint_in_fira`
    // in `shaper/tests.rs`) must still fail.
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
