use super::{LogicalSize, Scale};
use crate::{PhysicalSize, Point};

#[test]
fn scale_new_clamps_zero_to_one() {
    assert_eq!(Scale::new(0.0), Scale::ONE);
}

#[test]
fn scale_new_clamps_negative_to_one() {
    assert_eq!(Scale::new(-1.0), Scale::ONE);
}

#[test]
fn scale_new_clamps_nan_to_one() {
    assert_eq!(Scale::new(f32::NAN), Scale::ONE);
}

#[test]
fn to_physical_roundtrips_at_scale_2x() {
    assert_eq!(
        Scale::new(2.0).to_physical(LogicalSize::new(720.0, 480.0)),
        PhysicalSize::new(1440, 960)
    );
}

#[test]
fn to_physical_clamps_to_one_minimum() {
    assert_eq!(
        Scale::ONE.to_physical(LogicalSize::new(0.4, 0.4)),
        PhysicalSize::new(1, 1)
    );
}

#[test]
fn to_logical_roundtrips_at_scale_125() {
    assert_eq!(
        Scale::new(1.25).to_logical(PhysicalSize::new(900, 600)),
        LogicalSize::new(720.0, 480.0)
    );
}

#[test]
fn point_to_logical_divides_by_factor() {
    assert_eq!(
        Scale::new(2.0).point_to_logical(Point::new(150.0, 75.0)),
        Point::new(75.0, 37.5)
    );
}
