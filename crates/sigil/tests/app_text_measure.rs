#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::app::DefaultTextMeasure;
use sigil::geometry::{Dim, Size};
use sigil::kernel::TextContent;
use sigil::layout::TextMeasure;

#[test]
fn default_text_measure_reports_real_static_text_size() {
    let m = DefaultTextMeasure::new();
    let size = m.measure(
        &TextContent::Static("anything".to_string()),
        Size {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        },
    );

    assert!(size.width > 0.0, "{size:?}");
    assert!(size.height > 0.0, "{size:?}");
}

#[test]
fn default_text_measure_min_content_is_no_wider_than_max_content() {
    let m = DefaultTextMeasure::new();
    let content = TextContent::Static("short muchlongerword".to_string());
    let min = m.measure(
        &content,
        Size {
            width: Dim::MinContent,
            height: Dim::MinContent,
        },
    );
    let max = m.measure(
        &content,
        Size {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        },
    );

    assert!(min.width <= max.width, "min={min:?} max={max:?}");
}

#[test]
fn default_text_measure_definite_width_wraps_text() {
    let m = DefaultTextMeasure::new();
    let content = TextContent::Static("Hello from the launch text measurer".to_string());
    let wide = m.measure(
        &content,
        Size {
            width: Dim::Fixed(400.0),
            height: Dim::MaxContent,
        },
    );
    let narrow = m.measure(
        &content,
        Size {
            width: Dim::Fixed(40.0),
            height: Dim::MaxContent,
        },
    );

    assert!(
        narrow.height >= wide.height,
        "wide={wide:?} narrow={narrow:?}"
    );
}
