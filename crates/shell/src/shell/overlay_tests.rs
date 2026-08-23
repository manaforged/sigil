use super::overlay_origin;

#[test]
fn overlay_origin_is_parent_plus_scaled_hole() {
    assert_eq!(overlay_origin((100, 200), (10.0, 20.0), 1.0), (110, 220));
}

#[test]
fn overlay_origin_scales_logical_origin() {
    assert_eq!(overlay_origin((0, 0), (10.0, 20.0), 2.0), (20, 40));
}
