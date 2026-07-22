//! Core measurement correctness tests.

#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use ahash as _;
use criterion as _;
#[cfg(feature = "shaping-complex")]
use harfrust as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};
use smallvec as _;
use tracing as _;
use unicode_bidi as _;
use unicode_linebreak as _;
use unicode_script as _;
use unicode_segmentation as _;

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_style::{TextLineHeight, TextStyle};
use sigil_text::{OwnedTextEngine, TextConfig};

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

#[test]
fn non_empty_text_has_positive_dimensions() {
    let e = engine();
    let para = e.layout_paragraph("Hello", None).unwrap();
    assert!(para.width() > 0.0, "width should be positive");
    assert!(para.height() > 0.0, "height should be positive");
}

#[test]
fn different_strings_produce_different_widths() {
    let e = engine();
    let short = e.layout_paragraph("Hi", None).unwrap();
    let long = e.layout_paragraph("Hello world", None).unwrap();
    assert!(
        long.width() > short.width(),
        "longer text should be wider: {} vs {}",
        long.width(),
        short.width()
    );
}

#[test]
fn empty_string_has_zero_width() {
    let e = engine();
    let para = e.layout_paragraph("", None).unwrap();
    assert!(
        para.width() < f32::EPSILON,
        "empty text width: {}",
        para.width()
    );
}

#[test]
fn max_width_constraint_wraps_text() {
    let e = engine();
    let wide = e.layout_paragraph("Hello world", None).unwrap();
    let narrow = e.layout_paragraph("Hello world", Some(20.0)).unwrap();

    assert!(
        narrow.line_count() > wide.line_count(),
        "narrow constraint should produce more lines: {} vs {}",
        narrow.line_count(),
        wide.line_count()
    );
    assert!(
        narrow.height() > wide.height(),
        "wrapped text should be taller"
    );
}

#[test]
fn line_count_is_one_for_short_text() {
    let e = engine();
    let para = e.layout_paragraph("Hi", None).unwrap();
    assert_eq!(para.line_count(), 1);
}

/// UAX #14 glue: a no-break space (U+00A0) must keep its neighbours on
/// one line even when the same width wraps the ordinary-space version.
/// The pre-UAX#14 breaker treated U+00A0 as ordinary whitespace and
/// split it — this is the regression pin for that fix.
#[test]
fn nbsp_glues_across_a_wrap_boundary() {
    let e = engine();
    // Width that comfortably wraps "10 km" but is narrower than the full
    // run, so the ordinary space is the only thing that can break.
    let full = e.layout_paragraph("10 km", None).unwrap().width();
    let limit = Some(full * 0.6);

    let spaced = e.layout_paragraph("10 km", limit).unwrap();
    let glued = e.layout_paragraph("10\u{00A0}km", limit).unwrap();

    assert!(
        spaced.line_count() >= 2,
        "an ordinary space must wrap at this width, got {} line(s)",
        spaced.line_count()
    );
    assert_eq!(
        glued.line_count(),
        1,
        "U+00A0 is glue and must never introduce a soft break"
    );
}

/// UAX #14 hyphen: a break is permitted after U+002D, so a hyphenated
/// run wraps, while the same characters with no hyphen have no soft
/// break opportunity and overflow on a single line.
#[test]
fn hyphen_offers_a_break_but_solid_run_does_not() {
    let e = engine();
    let limit = Some(e.layout_paragraph("aaa-", None).unwrap().width() * 1.2);

    let hyphenated = e.layout_paragraph("aaa-bbb-ccc", limit).unwrap();
    let solid = e.layout_paragraph("aaabbbccc", limit).unwrap();

    assert!(
        hyphenated.line_count() >= 2,
        "a hyphenated run must wrap after the hyphen, got {} line(s)",
        hyphenated.line_count()
    );
    assert_eq!(
        solid.line_count(),
        1,
        "a run with no break opportunity overflows on one line"
    );
}

#[test]
fn styled_font_size_and_line_height_change_metrics() {
    let e = engine();
    let small = TextStyle {
        font_size: 12.0,
        line_height: TextLineHeight::Absolute(14.0),
        ..TextStyle::default()
    };
    let large = TextStyle {
        font_size: 24.0,
        line_height: TextLineHeight::Absolute(32.0),
        ..TextStyle::default()
    };

    let small = e
        .layout_paragraph_styled("Typography", None, &small)
        .unwrap();
    let large = e
        .layout_paragraph_styled("Typography", None, &large)
        .unwrap();

    assert!(
        large.width() > small.width(),
        "larger font should be wider: {} vs {}",
        large.width(),
        small.width()
    );
    assert!(
        large.height() > small.height(),
        "larger line height should be taller: {} vs {}",
        large.height(),
        small.height()
    );
}

/// The baseline must come from the face's real ascent/descent under the
/// CSS half-leading model, not a fixed fraction of the line box.
/// Expectation computed independently from the same font bytes via skrifa.
#[test]
fn baseline_is_half_leading_plus_real_ascent() {
    const FONT_SIZE: f32 = 16.0;
    const LINE_HEIGHT: f32 = 32.0;

    let bytes = std::fs::read("assets/fonts/FiraSans-Regular.ttf").unwrap();
    let font = FontRef::new(&bytes).unwrap();
    let m = font.metrics(Size::new(FONT_SIZE), LocationRef::default());
    let (ascent, descent) = (m.ascent, -m.descent);
    let expected = (LINE_HEIGHT - (ascent + descent)) / 2.0 + ascent;

    let mut e = engine();
    e.register_font(bytes.clone(), 0).unwrap();
    let style = TextStyle {
        font_size: FONT_SIZE,
        line_height: TextLineHeight::Absolute(LINE_HEIGHT),
        ..TextStyle::default()
    };
    let para = e.layout_paragraph_styled("A\ng", None, &style).unwrap();

    let glyphs = para.rendered_glyphs();
    assert_eq!(glyphs.len(), 2, "one glyph per line");
    assert!(
        (glyphs[0].y - expected).abs() < 1e-3,
        "line 0 baseline {} != half-leading + ascent {expected} (ascent {ascent}, descent {descent})",
        glyphs[0].y,
    );
    assert!(
        (glyphs[1].y - (expected + LINE_HEIGHT)).abs() < 1e-3,
        "line 1 baseline {} != {} (line 0 + line height)",
        glyphs[1].y,
        expected + LINE_HEIGHT,
    );
}
