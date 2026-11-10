#![allow(clippy::unwrap_used, clippy::float_cmp)]

use sigil::style::TextStyle;
use sigil::text::OwnedTextEngine;

const LABEL: &str = "Waiting for restock";

#[test]
fn a_line_that_fits_within_rounding_does_not_wrap() {
    let style = TextStyle::default();
    let unbounded = OwnedTextEngine::new(TextStyle {
        font_size: 16.0,
        ..TextStyle::default()
    })
    .layout_paragraph_styled(LABEL, None, &style)
    .unwrap();
    let fitted = OwnedTextEngine::new(TextStyle {
        font_size: 16.0,
        ..TextStyle::default()
    })
    .layout_paragraph_styled(LABEL, Some(unbounded.width() - 1e-4), &style)
    .unwrap();
    assert_eq!(fitted.height(), unbounded.height());
}
