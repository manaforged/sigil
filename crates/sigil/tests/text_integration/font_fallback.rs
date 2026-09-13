#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use std::collections::HashSet;

use sigil::hal::{ParagraphMetrics, TextEngine};
use sigil::text::{OwnedTextEngine, TextConfig};

const NOTO_HEBREW: &[u8] = include_bytes!("../fixtures/text/NotoSansHebrew-Regular.ttf");
const HEBREW_WORD: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

const ETHIOPIC_WORD: &str = "\u{1230}\u{120B}\u{121D}";

#[test]
fn without_any_fallback_face_a_script_renders_as_tofu() {
    let e = engine();
    let p = e.layout_paragraph(ETHIOPIC_WORD, None).unwrap();
    let glyphs = p.rendered_glyphs();
    assert!(
        glyphs.iter().any(|g| g.glyph_id == 0),
        "no bundled or system face covers Ethiopic; this must be .notdef tofu",
    );
}

#[test]
fn mixed_latin_hebrew_falls_back_to_registered_face() {
    let mut e = engine();
    e.register_font_family("serif", NOTO_HEBREW.to_vec(), 0)
        .expect("NotoSansHebrew fixture must register");

    let p = e
        .layout_paragraph(&format!("Hello {HEBREW_WORD}"), None)
        .unwrap();
    let glyphs = p.rendered_glyphs();
    assert!(!glyphs.is_empty(), "mixed paragraph must produce glyphs");
    assert!(
        glyphs.iter().all(|g| g.glyph_id != 0),
        "no glyph should be .notdef once a covering face is registered: {glyphs:?}",
    );

    let distinct_fonts: HashSet<u64> = glyphs.iter().map(|g| g.font_id).collect();
    assert_eq!(
        distinct_fonts.len(),
        2,
        "Latin run (FiraSans) and Hebrew run (fallback) must use two distinct font ids, \
         got {distinct_fonts:?}",
    );
}

#[test]
fn emoji_falls_back_to_the_bundled_color_face() {
    let e = engine();
    let p = e.layout_paragraph("A\u{2705}", None).unwrap();
    let glyphs = p.rendered_glyphs();

    assert!(!glyphs.is_empty(), "must produce glyphs");
    assert!(
        glyphs.iter().all(|g| g.glyph_id != 0),
        "the emoji must not be .notdef tofu — the bundled fallback covers it: {glyphs:?}",
    );
    let distinct_fonts: HashSet<u64> = glyphs.iter().map(|g| g.font_id).collect();
    assert_eq!(
        distinct_fonts.len(),
        2,
        "Latin 'A' (FiraSans) and ✅ (emoji fallback) must use two distinct font ids, \
         got {distinct_fonts:?}",
    );
}

#[test]
fn for_each_font_yields_every_face_a_glyph_actually_uses() {
    let mut e = engine();
    e.register_font_family("serif", NOTO_HEBREW.to_vec(), 0)
        .expect("NotoSansHebrew fixture must register");
    let p = e
        .layout_paragraph(&format!("Hi {HEBREW_WORD}"), None)
        .unwrap();

    let mut registered = HashSet::new();
    p.for_each_font(|font_id, bytes, _index| {
        assert!(!bytes.is_empty(), "font bytes must not be empty");
        registered.insert(font_id);
    });

    let glyph_font_ids: HashSet<u64> = p.rendered_glyphs().iter().map(|g| g.font_id).collect();
    assert!(
        glyph_font_ids.is_subset(&registered),
        "every font_id a rendered glyph references must be registered via for_each_font: \
         glyph ids {glyph_font_ids:?}, registered {registered:?}",
    );
    assert_eq!(
        glyph_font_ids.len(),
        2,
        "expect exactly the primary + one fallback face to be referenced, got {glyph_font_ids:?}",
    );
}

#[test]
fn fallback_face_registration_does_not_disturb_pure_latin_layout() {
    let e_baseline = engine();
    let baseline = e_baseline.layout_paragraph("Hello world", None).unwrap();

    let mut e_with_fallback = engine();
    e_with_fallback
        .register_font_family("serif", NOTO_HEBREW.to_vec(), 0)
        .unwrap();
    let with_fallback = e_with_fallback
        .layout_paragraph("Hello world", None)
        .unwrap();

    assert!(
        (baseline.width() - with_fallback.width()).abs() < f32::EPSILON,
        "registering an unrelated fallback face must not perturb Latin-only width: \
         {} vs {}",
        baseline.width(),
        with_fallback.width(),
    );
}
