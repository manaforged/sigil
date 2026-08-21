use super::*;

#[test]
fn physical_extent_scales_fractional_logical_bounds_outward() {
    let rect = Rect::new(Point::new(10.0, 20.0), 10.25, 5.25);
    assert_eq!(physical_extent(rect, 2.0), (21, 11));
}
