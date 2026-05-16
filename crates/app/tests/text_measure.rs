//! `DefaultTextMeasure` launch behaviour.

#![allow(missing_docs)]

use sigil_async as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use sigil_app::DefaultTextMeasure;
use sigil_kernel::TextContent;
use sigil_layout::TextMeasure;
use taffy::prelude::{AvailableSpace, Size as TaffySize};

#[test]
fn default_text_measure_reports_real_static_text_size() {
    let m = DefaultTextMeasure::new();
    let size = m.measure(
        &TextContent::Static("anything".to_string()),
        TaffySize {
            width: AvailableSpace::MaxContent,
            height: AvailableSpace::MaxContent,
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
        TaffySize {
            width: AvailableSpace::MinContent,
            height: AvailableSpace::MinContent,
        },
    );
    let max = m.measure(
        &content,
        TaffySize {
            width: AvailableSpace::MaxContent,
            height: AvailableSpace::MaxContent,
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
        TaffySize {
            width: AvailableSpace::Definite(400.0),
            height: AvailableSpace::MaxContent,
        },
    );
    let narrow = m.measure(
        &content,
        TaffySize {
            width: AvailableSpace::Definite(40.0),
            height: AvailableSpace::MaxContent,
        },
    );

    assert!(
        narrow.height >= wide.height,
        "wide={wide:?} narrow={narrow:?}"
    );
}
