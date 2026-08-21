use super::visible_band;

#[test]
fn band_accepts_gpui_page_and_sigil_overscan() {
    assert_eq!(visible_band(1_280.0, 32.0, 1_000_000), (40, 121));
    assert_eq!(visible_band(912.0, 24.0, 10_000), (38, 115));
    assert_eq!(visible_band(1_280.0, 32.0, 3), (3, 3));
}
