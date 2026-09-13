use super::*;

#[allow(
    clippy::float_cmp,
    reason = "exact comparison is the contract (regression pin)"
)]
#[test]
fn fixed_dimension_exposes_logical_pixels() {
    assert_eq!(Dimension::Px(Px(44)).px_or_zero(), 44.0);
    assert_eq!(Dimension::Full.px_or_zero(), 0.0);
}

#[allow(
    clippy::float_cmp,
    reason = "exact comparison is the contract (regression pin)"
)]
#[test]
fn fixed_length_exposes_logical_pixels() {
    assert_eq!(LengthPercentage::Px(Px(12)).px_or_zero(), 12.0);
    assert_eq!(LengthPercentage::Percent(0.5).px_or_zero(), 0.0);
}
