use super::CornerRadii;

#[test]
fn oversized_uniform_radius_becomes_capsule() {
    assert_radii_close(
        CornerRadii::uniform(9_999.0).normalized_for(80.0, 40.0),
        [20.0; 4],
    );
}

#[test]
fn asymmetric_radii_share_one_css_scale_factor() {
    assert_radii_close(
        CornerRadii::new(40.0, 20.0, 10.0, 30.0).normalized_for(30.0, 80.0),
        [20.0, 10.0, 5.0, 15.0],
    );
}

fn assert_radii_close(actual: [f32; 4], expected: [f32; 4]) {
    for (observed, wanted) in actual.into_iter().zip(expected) {
        assert!((observed - wanted).abs() < f32::EPSILON);
    }
}
