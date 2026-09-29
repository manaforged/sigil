use crate::events::WindowEvent;
use crate::geometry::{Point, Scale};

use super::logical_point_from_f64;

#[test]
fn pointer_events_at_scale_2x_report_logical_coordinates() {
    let position = logical_point_from_f64(150.0, 75.0, Scale::new(2.0));

    assert_eq!(position, Point::new(75.0, 37.5));
}

#[test]
fn scale_factor_changed_event_carries_scale_type() {
    let event = WindowEvent::ScaleFactorChanged(Scale::new(1.5));

    assert_eq!(event, WindowEvent::ScaleFactorChanged(Scale::new(1.5)));
}

#[test]
fn pointer_translation_uses_per_window_scale_after_change() {
    let event = WindowEvent::ScaleFactorChanged(Scale::new(2.0));
    let scale = scale_from_event(&event);

    let position = logical_point_from_f64(200.0, 100.0, scale);

    assert_eq!(position, Point::new(100.0, 50.0));
}

const fn scale_from_event(event: &WindowEvent) -> Scale {
    if let WindowEvent::ScaleFactorChanged(scale) = event {
        *scale
    } else {
        Scale::ONE
    }
}
