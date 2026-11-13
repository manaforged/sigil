#![allow(clippy::unwrap_used)]

use std::rc::Rc;

use sigil::geometry::{Constraint, Size};
use sigil::kernel::TextContent;
use sigil::layout::{TextEngineMeasure, TextMeasure};
use sigil::style::TextStyle;
use sigil::text::OwnedTextEngine;

fn make_measurer() -> TextEngineMeasure {
    let engine = Rc::new(OwnedTextEngine::new(TextStyle::default().font_size(16.0)));
    TextEngineMeasure::new(engine)
}

fn measure_with_space(measurer: &TextEngineMeasure, text: &str, space: Constraint) -> Size {
    let source = TextContent::Static(text.to_owned());
    measurer.measure(&source, space)
}

#[test]
fn max_content_returns_unconstrained_width() {
    let m = make_measurer();
    let size = measure_with_space(&m, "Hello world", Constraint::max_content());
    assert!(size.width > 0.0, "max content width should be positive");
}

#[test]
fn min_content_is_narrower_than_max_content() {
    let m = make_measurer();
    let max = measure_with_space(&m, "Hello world test string", Constraint::max_content());
    let min = measure_with_space(&m, "Hello world test string", Constraint::min_content());

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

    let text = "Hi Internationalization Go";
    let min = measure_with_space(&m, text, Constraint::min_content());
    let longest_alone = measure_with_space(&m, "Internationalization", Constraint::max_content());

    let tolerance = 1.0;
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
    let wide = measure_with_space(&m, text, Constraint::max_content());
    let narrow = measure_with_space(&m, text, Constraint::max_content().width(80.0));

    assert!(
        narrow.height > wide.height,
        "narrow constraint should wrap and be taller: {} vs {}",
        narrow.height,
        wide.height
    );
}
