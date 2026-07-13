//! Core measurement correctness tests.

#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use ahash as _;
use criterion as _;
#[cfg(feature = "shaping-complex")]
use harfrust as _;
use kernel as _;
use layout as _;
use reactive as _;
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};
use smallvec as _;
use tracing as _;
use unicode_bidi as _;
use unicode_script as _;
use unicode_segmentation as _;

use hal::{ParagraphMetrics, TextEngine};
use style::{TextLineHeight, TextStyle};
use text::{OwnedTextEngine, TextConfig};

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

    let bytes = std::fs::read("tests/fixtures/FiraSans-Regular.ttf").unwrap();
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
