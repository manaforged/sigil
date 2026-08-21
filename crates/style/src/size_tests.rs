use super::*;

// The two `px_or_zero` paths return either `int as f32` (bit-exact for
// values that fit in 24 bits — Px is i32 well within range) or the const
// `0.0`. Bit-exact equality is the correct assertion here.
#[allow(clippy::float_cmp)]
#[test]
fn fixed_dimension_exposes_logical_pixels() {
    assert_eq!(Dimension::Px(Px(44)).px_or_zero(), 44.0);
    assert_eq!(Dimension::Full.px_or_zero(), 0.0);
}

#[allow(clippy::float_cmp)]
#[test]
fn fixed_length_exposes_logical_pixels() {
    assert_eq!(LengthPercentage::Px(Px(12)).px_or_zero(), 12.0);
    assert_eq!(LengthPercentage::Percent(0.5).px_or_zero(), 0.0);
}
