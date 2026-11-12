use sigil::geometry::Size;
use sigil::kernel::TextContent;
use sigil::layout::TextEngineMeasure;
use sigil::layout::TextMeasure;

#[test]
fn default_text_measure_reports_real_static_text_size() {
    let m = TextEngineMeasure::default();
    let size = m.measure(
        &TextContent::Static("anything".to_string()),
        Size::max_content(),
    );

    assert!(size.width > 0.0, "{size:?}");
    assert!(size.height > 0.0, "{size:?}");
}

#[test]
fn default_text_measure_min_content_is_no_wider_than_max_content() {
    let m = TextEngineMeasure::default();
    let content = TextContent::Static("short muchlongerword".to_string());
    let min = m.measure(&content, Size::min_content());
    let max = m.measure(&content, Size::max_content());

    assert!(min.width <= max.width, "min={min:?} max={max:?}");
}

#[test]
fn default_text_measure_definite_width_wraps_text() {
    let m = TextEngineMeasure::default();
    let content = TextContent::Static("Hello from the launch text measurer".to_string());
    let wide = m.measure(&content, Size::max_content().width(400.0));
    let narrow = m.measure(&content, Size::max_content().width(40.0));

    assert!(
        narrow.height >= wide.height,
        "wide={wide:?} narrow={narrow:?}"
    );
}
