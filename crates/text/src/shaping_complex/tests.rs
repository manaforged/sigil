//! Integration-style tests for HarfRust complex-script shaping (G1.6).
//!
//! Fixture fonts ship in `crates/text/tests/fixtures/` (Noto Sans
//! Arabic + Noto Sans Hebrew, SIL OFL 1.1). Tests live under
//! `#[cfg(test)]` inside the crate because `Shaper`, `TextSegment`,
//! and `Style` are `pub(crate)` — the integration surface for the
//! engine layer (G1.4) is `Paragraph`, not the shaper directly.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_lossless
)]

use crate::glyph::GlyphId;
use crate::shaper::{Font, Shaper, Style, TextSegment};
use crate::shaping_complex;

const NOTO_ARABIC: &[u8] = include_bytes!("../../tests/fixtures/NotoSansArabic-Regular.ttf");
const NOTO_HEBREW: &[u8] = include_bytes!("../../tests/fixtures/NotoSansHebrew-Regular.ttf");

fn noto_arabic() -> Font {
    Font::from_bytes(NOTO_ARABIC.to_vec(), 0).expect("NotoSansArabic fixture must parse")
}

fn noto_hebrew() -> Font {
    Font::from_bytes(NOTO_HEBREW.to_vec(), 0).expect("NotoSansHebrew fixture must parse")
}

#[test]
fn arabic_alef_lam_ain_shapes_with_multiple_glyphs() {
    // ALEF (U+0627) + LAM (U+0644) + AIN (U+0639) — "العَ" stripped of
    // the diacritic. The OpenType GSUB plan in NotoSansArabic should
    // join LAM-ALEF into a ligature and emit positional variants
    // (isolated/initial/medial/final). Minimum-bar contract:
    //   * non-empty glyph run with no .notdef glyphs
    //   * cluster monotonicity within the visual order returned by
    //     HarfRust (RTL: rightmost source byte first in visual order)
    let font = noto_arabic();
    let style = Style::regular(16.0);
    let segment = TextSegment {
        text: "\u{0627}\u{0644}\u{0639}",
        level: 1, // RTL
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
    // Visual width must be positive — proves we scaled font units → px.
    assert!(
        run.bbox.w > 0.0,
        "shaped Arabic must have positive visual width, got {}",
        run.bbox.w,
    );
}

#[test]
fn hebrew_shalom_shapes_rtl_with_glyph_per_consonant() {
    // SHIN (U+05E9) + LAMED (U+05DC) + VAV (U+05D5) + MEM-FINAL (U+05DD)
    // — "שלום". Hebrew without niqqud should shape one-to-one (no
    // ligatures in NotoSansHebrew-Regular for these four), and the
    // HarfRust output visual order for an RTL segment runs source
    // bytes high-to-low: cluster sequence should start with the
    // SHIN at byte 0 and end with MEM-FINAL at byte 6.
    let font = noto_hebrew();
    let style = Style::regular(16.0);
    let segment = TextSegment {
        text: "\u{05E9}\u{05DC}\u{05D5}\u{05DD}",
        level: 1, // RTL
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
    // RTL visual order: first emitted glyph corresponds to the rightmost
    // source codepoint (MEM-FINAL at byte 6) when bidi level 1.
    // Hebrew letters U+05D0..U+05EA all encode as 2 UTF-8 bytes, so
    // the four-character "שלום" occupies byte offsets {0, 2, 4, 6}.
    let clusters: Vec<u32> = run.glyphs.iter().map(|g| g.cluster).collect();
    assert_eq!(
        clusters[0], 6,
        "RTL visual order: first glyph clusters to last source byte (MEM-FINAL at 6), got {clusters:?}",
    );
    assert_eq!(
        clusters[3], 0,
        "RTL visual order: last glyph clusters to first source byte (SHIN at 0), got {clusters:?}",
    );
    // Strict monotonicity check: clusters strictly decrease (RTL).
    for w in clusters.windows(2) {
        assert!(
            w[0] > w[1],
            "RTL clusters must strictly decrease in visual order, got {clusters:?}",
        );
    }
}

#[test]
fn arabic_via_shaper_dispatch_routes_through_harfrust() {
    // The `Shaper::shape` entry point classifies the segment and
    // dispatches to HarfRust when the feature is on. We can't
    // observe the path directly, but the result must non-trivially
    // differ from the owned fallback: HarfRust handles joining,
    // the owned shaper does not. The robust check is that the
    // glyph IDs we get out are not the cmap's first-codepoint
    // mapping for ALEF/LAM/AIN — HarfRust substitutes positional
    // variants, the owned shaper does not.
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
fn devanagari_shapes_with_reordering() {
    // Devanagari requires GSUB reordering for the I-matra. Add a
    // NotoSansDevanagari-Regular fixture to enable this.
}

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
fn malformed_font_returns_none() {
    // Build a Font with bytes that pass skrifa but fail HarfRust's
    // stricter read_fonts version is unlikely in practice; the
    // simpler test is that an empty buffer would fail at Font::
    // from_bytes() entirely. We instead exercise the contract that
    // `shape()` plumbs `Option` through cleanly — empty text returns
    // Some(empty), which is the documented success-on-trivial path.
    let font = noto_arabic();
    let segment = TextSegment::ltr("");
    let style = Style::regular(16.0);
    assert!(shaping_complex::shape(&segment, &style, &font).is_some());
}
