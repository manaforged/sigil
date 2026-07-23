//! Single-line ellipsis truncation (`layout_paragraph_ellipsized`).

#![allow(missing_docs, clippy::unwrap_used)]

use ahash as _;
use criterion as _;
#[cfg(feature = "shaping-complex")]
use harfrust as _;
use image as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use unicode_bidi as _;
use unicode_linebreak as _;
use unicode_script as _;
use unicode_segmentation as _;

use sigil_hal::ParagraphMetrics;
use sigil_style::TextStyle;
use sigil_text::{OwnedTextEngine, TextConfig};

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

#[test]
fn short_text_is_returned_untruncated() {
    let e = engine();
    let style = TextStyle::default();
    let natural = e
        .layout_paragraph_styled("Hi", None, &style)
        .unwrap()
        .width();
    // A generous budget: the text fits, so it must come back unchanged.
    let para = e
        .layout_paragraph_ellipsized("Hi", natural + 100.0, &style)
        .unwrap();
    assert!(
        (para.width() - natural).abs() < 0.01,
        "text that fits must not be truncated; got {} vs natural {natural}",
        para.width(),
    );
}

#[test]
fn long_text_is_truncated_to_fit_the_budget() {
    let e = engine();
    let style = TextStyle::default();
    let long = "This is a very long single-line label that overflows its box";
    let full = e
        .layout_paragraph_styled(long, None, &style)
        .unwrap()
        .width();
    let budget = full / 3.0;
    let para = e.layout_paragraph_ellipsized(long, budget, &style).unwrap();
    assert!(
        para.width() <= budget + 0.5,
        "ellipsized width {} must fit the {budget} budget",
        para.width(),
    );
    // It must still draw *something* — not collapse to empty — and be a
    // single line (ellipsis keeps one line, never wraps).
    assert!(para.width() > 0.0, "ellipsized text must be non-empty");
    assert_eq!(para.line_count(), 1, "ellipsis stays on one line");
}

#[test]
fn budget_smaller_than_ellipsis_still_returns_one_line() {
    let e = engine();
    let style = TextStyle::default();
    // A budget below even the ellipsis glyph: degrade to the bare ellipsis
    // rather than panicking or looping.
    let para = e
        .layout_paragraph_ellipsized("some text", 0.5, &style)
        .unwrap();
    assert_eq!(para.line_count(), 1, "degrades to a single ellipsis line");
}
