//! Core measurement correctness tests.

#![allow(missing_docs, clippy::unwrap_used)]

use ahash as _;
use criterion as _;
use parley as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use skrifa as _;
use smallvec as _;
use taffy as _;
use unicode_bidi as _;
use unicode_segmentation as _;

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_style::{TextLineHeight, TextStyle};
use sigil_text::{ParleyTextEngine, TextConfig};

fn engine() -> ParleyTextEngine {
    ParleyTextEngine::new(TextConfig::default())
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
