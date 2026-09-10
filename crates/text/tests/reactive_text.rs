#![allow(missing_docs, clippy::unwrap_used)]

use std::rc::Rc;

use sigil_hal::{Dim, Size};
use sigil_kernel::TextContent;
use sigil_layout::TextEngineMeasure;
use sigil_layout::TextMeasure;
use sigil_reactive::{Runtime, signal};
use sigil_text::{OwnedTextEngine, TextConfig};

fn make_measurer() -> TextEngineMeasure<OwnedTextEngine> {
    let engine = Rc::new(OwnedTextEngine::new(TextConfig::default()));
    TextEngineMeasure::new(engine)
}

#[test]
fn reactive_text_measures_current_value() {
    Runtime::scope("reactive_measure", || {
        let sig = signal(String::from("Hello"));
        let source = TextContent::Reactive(sig);
        let measurer = make_measurer();

        let size = measurer.measure(
            &source,
            Size {
                width: Dim::MaxContent,
                height: Dim::MaxContent,
            },
        );

        assert!(size.width > 0.0, "reactive text should have width");
        assert!(size.height > 0.0, "reactive text should have height");
    });
}

#[test]
fn reactive_text_reflects_signal_update() {
    Runtime::scope("reactive_update", || {
        let sig = signal(String::from("Hi"));
        let source = TextContent::Reactive(sig);
        let measurer = make_measurer();

        let short = measurer.measure(
            &source,
            Size {
                width: Dim::MaxContent,
                height: Dim::MaxContent,
            },
        );

        sig.set(String::from("Hello world this is longer"));
        measurer.begin_frame();

        let long = measurer.measure(
            &source,
            Size {
                width: Dim::MaxContent,
                height: Dim::MaxContent,
            },
        );

        assert!(
            long.width > short.width,
            "longer text should be wider: {} vs {}",
            long.width,
            short.width
        );
    });
}
