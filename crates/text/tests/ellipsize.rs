#![allow(missing_docs, clippy::unwrap_used)]

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
    assert!(para.width() > 0.0, "ellipsized text must be non-empty");
    assert_eq!(para.line_count(), 1, "ellipsis stays on one line");
}

#[test]
fn budget_smaller_than_ellipsis_still_returns_one_line() {
    let e = engine();
    let style = TextStyle::default();
    let para = e
        .layout_paragraph_ellipsized("some text", 0.5, &style)
        .unwrap();
    assert_eq!(para.line_count(), 1, "degrades to a single ellipsis line");
}
