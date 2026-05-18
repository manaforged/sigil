//! Unit tests for the skrifa-backed shaper (G1.3).
//!
//! Fixture fonts live in `crates/text/tests/fixtures/` and are
//! `include_bytes!`-loaded so the unit tests run from any cwd.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_lossless
)]

use smallvec::SmallVec;

use super::{
    CodepointClass, FeatureTag, Font, Script, Shaper, Style, TextSegment, classify,
    classify_segment_script,
};
use crate::glyph::GlyphId;

const FIRA_REGULAR: &[u8] = include_bytes!("../../tests/fixtures/FiraSans-Regular.ttf");
const FIRA_BOLD: &[u8] = include_bytes!("../../tests/fixtures/FiraSans-Bold.ttf");
const FIRA_ITALIC: &[u8] = include_bytes!("../../tests/fixtures/FiraSans-Italic.ttf");
const NOTO_REGULAR: &[u8] = include_bytes!("../../tests/fixtures/NotoSans-Regular.ttf");

fn fira_regular() -> Font {
    Font::from_bytes(FIRA_REGULAR.to_vec(), 0).expect("FiraSans-Regular fixture must parse")
}

fn fira_bold() -> Font {
    Font::from_bytes(FIRA_BOLD.to_vec(), 0).expect("FiraSans-Bold fixture must parse")
}

fn fira_italic() -> Font {
    Font::from_bytes(FIRA_ITALIC.to_vec(), 0).expect("FiraSans-Italic fixture must parse")
}

fn noto_regular() -> Font {
    Font::from_bytes(NOTO_REGULAR.to_vec(), 0).expect("NotoSans-Regular fixture must parse")
}

#[test]
fn empty_input_yields_empty_run() {
    let font = fira_regular();
    let shaper = Shaper::new(&font);
    let run = shaper.shape(&TextSegment::ltr(""), &Style::regular(16.0));
    assert!(run.glyphs.is_empty(), "empty input must produce no glyphs");
    assert_eq!(run.bbox.w, 0.0);
}

#[test]
fn single_ascii_char_maps_to_real_glyph() {
    let font = fira_regular();
    let shaper = Shaper::new(&font);
    let run = shaper.shape(&TextSegment::ltr("a"), &Style::regular(16.0));
    assert_eq!(run.glyphs.len(), 1);
    let g = &run.glyphs[0];
    assert_ne!(g.id, GlyphId(0), "'a' must not be .notdef in FiraSans");
    assert!(
        g.advance > 0.0,
        "advance must be positive, got {}",
        g.advance
    );
    assert_eq!(g.cluster, 0);
}

#[test]
fn hello_word_is_five_glyphs_monotonic() {
    let font = fira_regular();
    let shaper = Shaper::new(&font);
    let run = shaper.shape(&TextSegment::ltr("hello"), &Style::regular(16.0));
    assert_eq!(run.glyphs.len(), 5, "5 ASCII chars → 5 glyphs");
    for (i, g) in run.glyphs.iter().enumerate() {
        assert_eq!(g.cluster, i as u32, "cluster {i} must equal byte offset");
        assert_ne!(g.id, GlyphId(0));
        assert!(g.advance > 0.0);
    }
    // bbox width must equal sum of advances (no kerning at G1.3).
    let total: f32 = run.glyphs.iter().map(|g| g.advance).sum();
    assert!(
        (run.bbox.w - total).abs() < 0.01,
        "bbox.w {} should equal Σadvance {}",
        run.bbox.w,
        total,
    );
}

#[test]
fn size_doubles_advances_scale_proportional() {
    let font = fira_regular();
    let shaper = Shaper::new(&font);
    let run_16 = shaper.shape(&TextSegment::ltr("hello"), &Style::regular(16.0));
    let run_32 = shaper.shape(&TextSegment::ltr("hello"), &Style::regular(32.0));
    for (g16, g32) in run_16.glyphs.iter().zip(run_32.glyphs.iter()) {
        let ratio = g32.advance / g16.advance;
        assert!(
            (ratio - 2.0).abs() < 0.02,
            "advance must double when ppem doubles, got ratio {ratio}",
        );
    }
}

#[test]
fn bold_glyph_metrics_differ_from_regular() {
    // Same codepoint, different fonts — bold typically resolves to the
    // same glyph index *within* its own font, but the advance and the
    // per-font glyph id stream differ. The robust check is that bold
    // advance is ≥ regular advance for at least one letter in a word.
    let reg_font = fira_regular();
    let bold_font = fira_bold();
    let reg = Shaper::new(&reg_font).shape(&TextSegment::ltr("Hello"), &Style::regular(16.0));
    let bold = Shaper::new(&bold_font).shape(
        &TextSegment::ltr("Hello"),
        &Style {
            size_px: 16.0,
            weight: 700,
            italic: false,
            features: SmallVec::new(),
        },
    );
    assert_eq!(reg.glyphs.len(), bold.glyphs.len());
    let some_wider = reg
        .glyphs
        .iter()
        .zip(bold.glyphs.iter())
        .any(|(r, b)| b.advance > r.advance);
    assert!(
        some_wider,
        "at least one bold glyph must be wider than the regular equivalent",
    );
}

#[test]
fn italic_glyph_stream_differs_from_regular() {
    let reg_font = fira_regular();
    let it_font = fira_italic();
    let reg = Shaper::new(&reg_font).shape(&TextSegment::ltr("italic"), &Style::regular(16.0));
    let it = Shaper::new(&it_font).shape(
        &TextSegment::ltr("italic"),
        &Style {
            size_px: 16.0,
            weight: 400,
            italic: true,
            features: SmallVec::new(),
        },
    );
    assert_eq!(reg.glyphs.len(), it.glyphs.len());
    // At least one glyph id must differ — italic 'a' is a stylistic
    // variant in FiraSans-Italic so the cmap maps to a different gid
    // than FiraSans-Regular for some letters (or, at minimum, the
    // advances differ as a quick robustness check).
    let any_diff = reg
        .glyphs
        .iter()
        .zip(it.glyphs.iter())
        .any(|(r, i)| r.id != i.id || (r.advance - i.advance).abs() > 0.01);
    assert!(any_diff, "italic must produce a different glyph stream");
}

#[test]
fn notdef_for_unknown_codepoint_in_fira() {
    // FiraSans does not cover U+5B57 (漢-related ideograph), so the
    // shaper must return .notdef glyph id 0 and advance 0.
    let font = fira_regular();
    let run = Shaper::new(&font).shape(&TextSegment::ltr("\u{5B57}"), &Style::regular(16.0));
    assert_eq!(run.glyphs.len(), 1);
    assert_eq!(run.glyphs[0].id, GlyphId(0), "missing codepoint → .notdef");
}

#[test]
#[ignore = "G1.3 followup: needs NotoSansCJK fixture font (current NotoSans-Regular is Latin/extended-Latin only). Logic + classify() path are covered by classify_emoji_block_check; unignore after adding crates/text/tests/fixtures/NotoSansCJKjp-Regular.otf."]
fn cjk_punctuation_classifies_and_routes_through_noto() {
    // U+3001 IDEOGRAPHIC COMMA — NotoSansCJK covers it.
    assert_eq!(classify('\u{3001}'), CodepointClass::CjkPunctuation);
    let font = noto_regular();
    let run = Shaper::new(&font).shape(&TextSegment::ltr("\u{3001}"), &Style::regular(16.0));
    assert_eq!(run.glyphs.len(), 1);
    assert_ne!(
        run.glyphs[0].id,
        GlyphId(0),
        "U+3001 must map in NotoSans (covered range)",
    );
    assert!(run.glyphs[0].advance > 0.0);
}

#[test]
fn classify_emoji_block_check() {
    assert_eq!(classify('\u{1F389}'), CodepointClass::Emoji); // 🎉
    assert_eq!(classify('\u{2603}'), CodepointClass::Emoji); // ☃
    assert_eq!(classify('a'), CodepointClass::Regular);
    assert_eq!(classify('\u{3000}'), CodepointClass::CjkPunctuation);
    assert_eq!(classify('\u{FF01}'), CodepointClass::HalfFullWidth);
}

#[test]
fn emoji_passthrough_produces_glyph_with_correct_cluster() {
    // The fixture fonts don't include emoji glyphs, so the shaper
    // routes 🎉 (4 UTF-8 bytes, one grapheme cluster) to .notdef but
    // still emits a single glyph with cluster=0 — the R7 atlas (G1.5)
    // routes off this gid.
    let font = fira_regular();
    let run = Shaper::new(&font).shape(&TextSegment::ltr("\u{1F389}"), &Style::regular(16.0));
    assert_eq!(
        run.glyphs.len(),
        1,
        "single emoji = single grapheme = 1 glyph"
    );
    assert_eq!(run.glyphs[0].cluster, 0);
}

#[test]
#[ignore = "G1.3 followup: needs NotoSansCJK fixture (see cjk_punctuation_classifies_and_routes_through_noto)."]
fn mixed_latin_and_cjk_punctuation_cluster_order() {
    let font = noto_regular();
    let run = Shaper::new(&font).shape(&TextSegment::ltr("a\u{3002}b"), &Style::regular(16.0));
    // 3 grapheme clusters: 'a' at 0, '。' (3 UTF-8 bytes) at 1, 'b' at 4.
    assert_eq!(run.glyphs.len(), 3);
    assert_eq!(run.glyphs[0].cluster, 0);
    assert_eq!(run.glyphs[1].cluster, 1);
    assert_eq!(run.glyphs[2].cluster, 4);
    for g in &run.glyphs {
        assert_ne!(g.id, GlyphId(0), "NotoSans covers 'a', '。', 'b'");
    }
}

#[test]
fn combining_mark_clusters_with_base_glyph() {
    // 'e' + COMBINING ACUTE ACCENT (U+0301) is one grapheme cluster.
    // The shaper emits 2 glyph positions sharing cluster=0; the
    // combining mark carries zero advance.
    let font = fira_regular();
    let run = Shaper::new(&font).shape(&TextSegment::ltr("e\u{0301}"), &Style::regular(16.0));
    assert_eq!(run.glyphs.len(), 2, "base + combining = 2 glyph positions");
    assert_eq!(run.glyphs[0].cluster, 0);
    assert_eq!(run.glyphs[1].cluster, 0, "mark shares base's cluster");
    assert!(run.glyphs[0].advance > 0.0);
    assert_eq!(run.glyphs[1].advance, 0.0, "combining mark = 0 advance");
}

#[test]
fn ltr_segment_clusters_are_byte_indices_monotonic() {
    let font = fira_regular();
    let input = "abc def";
    let run = Shaper::new(&font).shape(&TextSegment::ltr(input), &Style::regular(16.0));
    assert_eq!(run.glyphs.len(), 7);
    for (i, g) in run.glyphs.iter().enumerate() {
        assert_eq!(g.cluster, i as u32);
    }
    let clusters: Vec<u32> = run.glyphs.iter().map(|g| g.cluster).collect();
    assert!(
        clusters.windows(2).all(|w| w[0] <= w[1]),
        "LTR clusters must be non-decreasing, got {clusters:?}",
    );
}

#[test]
fn shaper_is_send_sync() {
    // Static assertion — fails to compile if Shaper loses Send + Sync.
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Shaper<'_>>();
    assert_send_sync::<Font>();
}

#[test]
fn feature_tag_round_trips_bytes() {
    // Style carries OT feature tags so G1.4 engine can request liga/kern.
    let tag = FeatureTag(*b"liga");
    assert_eq!(tag.0, *b"liga");
}

#[test]
fn invalid_font_data_is_rejected() {
    let bogus = vec![0u8; 64];
    let err = Font::from_bytes(bogus, 0).expect_err("bogus bytes must fail");
    assert_eq!(err, super::ShaperError::InvalidFont);
}

#[test]
fn classify_segment_script_covers_g16_routing_cases() {
    // ASCII Latin
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("abc")),
        Script::Latin,
    );
    // CJK ideograph U+6F22 (漢)
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{6F22}")),
        Script::Cjk,
    );
    // Arabic ALEF U+0627
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{0627}")),
        Script::Arabic,
    );
    // Hebrew ALEF U+05D0
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{05D0}")),
        Script::Hebrew,
    );
    // Devanagari KA U+0915
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{0915}")),
        Script::Devanagari,
    );
    // Thai SARA A U+0E32
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{0E32}")),
        Script::Thai,
    );
    // Khmer KA U+1780
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{1780}")),
        Script::Khmer,
    );
    // Lao KO U+0E81
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("\u{0E81}")),
        Script::Lao,
    );
    // Empty segment classifies as Common (default)
    assert_eq!(
        classify_segment_script(&TextSegment::ltr("")),
        Script::Common,
    );
    // Leading whitespace (Common) skipped, decisive char is Arabic
    assert_eq!(
        classify_segment_script(&TextSegment::ltr(" \u{0627}")),
        Script::Arabic,
    );
}

#[test]
fn script_is_complex_matches_g16_dispatch_set() {
    assert!(Script::Arabic.is_complex());
    assert!(Script::Hebrew.is_complex());
    assert!(Script::Devanagari.is_complex());
    assert!(Script::Khmer.is_complex());
    assert!(Script::Thai.is_complex());
    assert!(Script::Lao.is_complex());
    assert!(!Script::Latin.is_complex());
    assert!(!Script::Cjk.is_complex());
    assert!(!Script::Common.is_complex());
    assert!(!Script::Other.is_complex());
}
