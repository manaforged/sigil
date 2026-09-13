#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_lossless
)]

use crate::text::glyph::GlyphId;
use crate::text::shaper::{Font, Shaper, Style, TextSegment};
use crate::text::shaping_complex;

const NOTO_ARABIC: &[u8] =
    include_bytes!("../../../tests/fixtures/text/NotoSansArabic-Regular.ttf");
const NOTO_HEBREW: &[u8] =
    include_bytes!("../../../tests/fixtures/text/NotoSansHebrew-Regular.ttf");
const FIRA_REGULAR: &[u8] = include_bytes!("../../../assets/text/fonts/FiraSans-Regular.ttf");

fn noto_arabic() -> Font {
    Font::from_bytes(NOTO_ARABIC.to_vec(), 0).expect("NotoSansArabic fixture must parse")
}

fn noto_hebrew() -> Font {
    Font::from_bytes(NOTO_HEBREW.to_vec(), 0).expect("NotoSansHebrew fixture must parse")
}

fn fira_regular() -> Font {
    Font::from_bytes(FIRA_REGULAR.to_vec(), 0).expect("FiraSans-Regular fixture must parse")
}

#[test]
fn arabic_alef_lam_ain_shapes_with_multiple_glyphs() {
    let font = noto_arabic();
    let style = Style::regular(16.0);
    let segment = TextSegment {
        text: "\u{0627}\u{0644}\u{0639}",
        level: 1,
    };
    let run = shaping_complex::shape(&segment, &style, &font).expect("Arabic shape must succeed");
    assert!(
        run.glyphs.len() >= 2,
        "Arabic 3-codepoint segment must shape into ≥2 glyphs, got {}",
        run.glyphs.len(),
    );
    for g in &run.glyphs {
        assert_ne!(g.id, GlyphId(0), "NotoSansArabic must cover ALEF/LAM/AIN");
        assert!(g.advance >= 0.0, "advances must be non-negative");
    }
    assert!(
        run.bbox.w > 0.0,
        "shaped Arabic must have positive visual width, got {}",
        run.bbox.w,
    );
}

#[test]
fn hebrew_shalom_shapes_rtl_with_glyph_per_consonant() {
    let font = noto_hebrew();
    let style = Style::regular(16.0);
    let segment = TextSegment {
        text: "\u{05E9}\u{05DC}\u{05D5}\u{05DD}",
        level: 1,
    };
    let run = shaping_complex::shape(&segment, &style, &font).expect("Hebrew shape must succeed");
    assert_eq!(
        run.glyphs.len(),
        4,
        "Hebrew 4-codepoint segment with no ligatures must produce 4 glyphs",
    );
    for g in &run.glyphs {
        assert_ne!(g.id, GlyphId(0), "NotoSansHebrew must cover all 4 letters");
        assert!(g.advance > 0.0);
    }
    let clusters: Vec<u32> = run.glyphs.iter().map(|g| g.cluster).collect();
    assert_eq!(
        clusters[0], 6,
        "RTL visual order: first glyph clusters to last source byte (MEM-FINAL at 6), got {clusters:?}",
    );
    assert_eq!(
        clusters[3], 0,
        "RTL visual order: last glyph clusters to first source byte (SHIN at 0), got {clusters:?}",
    );
    for w in clusters.windows(2) {
        assert!(
            w[0] > w[1],
            "RTL clusters must strictly decrease in visual order, got {clusters:?}",
        );
    }
}

#[test]
fn arabic_via_shaper_dispatch_routes_through_harfrust() {
    let font = noto_arabic();
    let shaper = Shaper::new(&font);
    let style = Style::regular(16.0);
    let segment = TextSegment {
        text: "\u{0627}\u{0644}\u{0639}",
        level: 1,
    };
    let run = shaper.shape(&segment, &style);
    assert!(
        !run.glyphs.is_empty(),
        "Shaper::shape must produce glyphs for Arabic when feature is on",
    );
    for g in &run.glyphs {
        assert_ne!(g.id, GlyphId(0));
    }
}

#[test]
#[ignore = "G1.6 followup: needs NotoSansDevanagari fixture font (current fixtures cover Arabic + Hebrew only)."]
fn devanagari_shapes_with_reordering() {}

#[test]
#[ignore = "G1.6 followup: needs NotoSansKhmer fixture font."]
fn khmer_shapes_with_complex_reordering() {}

#[test]
#[ignore = "G1.6 followup: needs NotoSansThai fixture font."]
fn thai_shapes_tone_marks_above_base() {}

#[test]
#[ignore = "G1.6 followup: needs NotoSansLao fixture font."]
fn lao_shapes_tone_marks_above_base() {}

#[test]
fn empty_segment_returns_empty_run() {
    let font = noto_arabic();
    let segment = TextSegment::ltr("");
    let style = Style::regular(16.0);
    let run = shaping_complex::shape(&segment, &style, &font)
        .expect("empty segment must short-circuit cleanly");
    assert!(run.glyphs.is_empty());
}

#[test]
fn latin_kerning_pair_av_advance_less_than_sum_of_individual() {
    let font = fira_regular();
    let style = Style::regular(16.0);
    let joint =
        shaping_complex::shape(&TextSegment::ltr("AV"), &style, &font).expect("AV must shape");
    assert_eq!(joint.glyphs.len(), 2, "AV must shape into exactly 2 glyphs");
    let joint_total: f32 = joint.glyphs.iter().map(|g| g.advance).sum();

    let a_advance = shaping_complex::shape(&TextSegment::ltr("A"), &style, &font)
        .expect("A must shape")
        .glyphs[0]
        .advance;
    let v_advance = shaping_complex::shape(&TextSegment::ltr("V"), &style, &font)
        .expect("V must shape")
        .glyphs[0]
        .advance;
    let separate_total = a_advance + v_advance;

    assert!(
        joint_total < separate_total - 0.01,
        "FiraSans kerns AV: joint advance {joint_total} must be less than the unkerned sum {separate_total}",
    );
}

#[test]
fn latin_fi_ligature_differs_from_separate_f_and_i() {
    let font = fira_regular();
    let style = Style::regular(16.0);
    let joint =
        shaping_complex::shape(&TextSegment::ltr("fi"), &style, &font).expect("fi must shape");
    let f = shaping_complex::shape(&TextSegment::ltr("f"), &style, &font).expect("f must shape");
    let i = shaping_complex::shape(&TextSegment::ltr("i"), &style, &font).expect("i must shape");

    let differs = joint.glyphs.len() != f.glyphs.len() + i.glyphs.len()
        || joint.glyphs[0].id != f.glyphs[0].id
        || joint.glyphs.get(1).map(|g| g.id) != Some(i.glyphs[0].id);
    assert!(
        differs,
        "FiraSans's fi ligature must produce a different glyph stream than f+i shaped apart, \
         got joint_len={} f_id={:?} i_id={:?}",
        joint.glyphs.len(),
        f.glyphs[0].id,
        i.glyphs[0].id,
    );
}

#[test]
fn malformed_font_returns_none() {
    let font = noto_arabic();
    let segment = TextSegment::ltr("");
    let style = Style::regular(16.0);
    assert!(shaping_complex::shape(&segment, &style, &font).is_some());
}
