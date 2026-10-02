#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use sigil::hal::ParagraphMetrics as _;
use sigil::style::TextStyle;
use sigil::text::{OwnedTextEngine, TextConfig};

const LABEL: &str = "Waiting for restock";

#[test]
fn a_line_that_fits_within_rounding_does_not_wrap() {
    let style = TextStyle::default();
    let unbounded = OwnedTextEngine::new(TextConfig::default())
        .layout_paragraph_styled(LABEL, None, &style)
        .unwrap();
    let fitted = OwnedTextEngine::new(TextConfig::default())
        .layout_paragraph_styled(LABEL, Some(unbounded.width() - 1e-4), &style)
        .unwrap();
    assert_eq!(fitted.height(), unbounded.height());
}
