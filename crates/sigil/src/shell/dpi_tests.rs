use crate::geometry::{Point, Scale};

use super::logical_point_from_f64;

#[test]
fn pointer_events_at_scale_2x_report_logical_coordinates() {
    let position = logical_point_from_f64(150.0, 75.0, Scale::new(2.0));

    assert_eq!(position, Point::new(75.0, 37.5));
}
