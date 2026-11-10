#![allow(clippy::unwrap_used)]

use std::rc::Rc;

use sigil::geometry::Size;
use sigil::kernel::TextContent;
use sigil::layout::{TextEngineMeasure, TextMeasure};
use sigil::reactive::{Owner, signal};
use sigil::style::TextStyle;
use sigil::text::OwnedTextEngine;

fn make_measurer() -> TextEngineMeasure {
    let engine = Rc::new(OwnedTextEngine::new(TextStyle {
        font_size: 16.0,
        ..TextStyle::default()
    }));
    TextEngineMeasure::new(engine)
}

#[test]
fn reactive_text_measures_current_value() {
    Owner::scope("reactive_measure", || {
        let sig = signal(String::from("Hello"));
        let source = TextContent::Reactive(sig);
        let measurer = make_measurer();

        let size = measurer.measure(&source, Size::max_content());

        assert!(size.width > 0.0, "reactive text should have width");
        assert!(size.height > 0.0, "reactive text should have height");
    });
}

#[test]
fn reactive_text_reflects_signal_update() {
    Owner::scope("reactive_update", || {
        let sig = signal(String::from("Hi"));
        let source = TextContent::Reactive(sig);
        let measurer = make_measurer();

        let short = measurer.measure(&source, Size::max_content());

        sig.set(String::from("Hello world this is longer"));
        measurer.begin_frame();

        let long = measurer.measure(&source, Size::max_content());

        assert!(
            long.width > short.width,
            "longer text should be wider: {} vs {}",
            long.width,
            short.width
        );
    });
}
