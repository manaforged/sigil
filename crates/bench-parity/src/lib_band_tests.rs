use super::{WorkloadParams, visible_band, visible_page, visible_span};

fn params(reuse: bool, vh: f32, rh: f32, rows: usize) -> WorkloadParams {
    WorkloadParams {
        row_count: rows,
        row_height_px: rh,
        viewport_width_px: 1_280.0,
        viewport_height_px: vh,
        row_shape: String::new(),
        scroll_pattern: String::from("linear"),
        iterations: 1,
        warmup: 0,
        surface: String::from("layout"),
        reuse,
        cells_per_row: 1,
    }
}

#[test]
fn band_accepts_gpui_page_and_sigil_overscan() {
    assert_eq!(visible_page(1_280.0, 32.0, 1_000_000), 40);
    assert_eq!(visible_band(1_280.0, 32.0, 1_000_000), (40, 121));
    assert_eq!(visible_band(912.0, 24.0, 10_000), (38, 115));
    assert_eq!(visible_band(1_280.0, 32.0, 3), (3, 3));
}

#[test]
fn construct_span_is_the_geometric_page() {
    assert_eq!(
        visible_span(&params(false, 1_280.0, 32.0, 100_000)),
        (40, 51)
    );
    assert_eq!(visible_span(&params(false, 912.0, 24.0, 10_000)), (38, 48));
    assert_eq!(visible_span(&params(true, 912.0, 24.0, 10_000)), (38, 133));
}
