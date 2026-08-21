//! `MinContent` / `MaxContent` width tests.

#![allow(missing_docs, clippy::unwrap_used)]

use std::rc::Rc;

use sigil_hal::{Dim, LogicalSize, Size};
use sigil_kernel::TextContent;
use sigil_layout::TextEngineMeasure;
use sigil_layout::TextMeasure;
use sigil_text::{OwnedTextEngine, TextConfig};

fn make_measurer() -> TextEngineMeasure<OwnedTextEngine> {
    let engine = Rc::new(OwnedTextEngine::new(TextConfig::default()));
    TextEngineMeasure::new(engine)
}

fn measure_with_space(
    measurer: &TextEngineMeasure<OwnedTextEngine>,
    text: &str,
    width: Dim,
) -> LogicalSize {
    let source = TextContent::Static(text.to_owned());
    measurer.measure(
        &source,
        Size {
            width,
            height: Dim::MaxContent,
        },
    )
}

#[test]
fn max_content_returns_unconstrained_width() {
    let m = make_measurer();
    let size = measure_with_space(&m, "Hello world", Dim::MaxContent);
    assert!(size.width > 0.0, "max content width should be positive");
}

#[test]
fn min_content_is_narrower_than_max_content() {
    let m = make_measurer();
    let max = measure_with_space(&m, "Hello world test string", Dim::MaxContent);
    let min = measure_with_space(&m, "Hello world test string", Dim::MinContent);

    assert!(
        min.width <= max.width,
        "min content should be <= max content: {min_w} vs {max_w}",
        min_w = min.width,
        max_w = max.width
    );
}

#[test]
fn min_content_equals_longest_word_width() {
    let m = make_measurer();

    // "Hi x Internationalization x Go" — the longest word is
    // "Internationalization". MinContent should equal the width of
    // that word laid out alone (since it's the widest unbreakable
    // segment).
    let text = "Hi Internationalization Go";
    let min = measure_with_space(&m, text, Dim::MinContent);
    let longest_alone = measure_with_space(&m, "Internationalization", Dim::MaxContent);

    let tolerance = 1.0; // sub-pixel rounding
    assert!(
        (min.width - longest_alone.width).abs() < tolerance,
        "min content ({min_w}) should match longest word width ({word_w})",
        min_w = min.width,
        word_w = longest_alone.width,
    );
}

#[test]
fn definite_width_produces_more_lines_than_unconstrained() {
    let m = make_measurer();
    let text = "The quick brown fox jumps over the lazy dog";
    let wide = measure_with_space(&m, text, Dim::MaxContent);
    let narrow = measure_with_space(&m, text, Dim::Fixed(80.0));

    assert!(
        narrow.height > wide.height,
        "narrow constraint should wrap and be taller: {} vs {}",
        narrow.height,
        wide.height
    );
}
