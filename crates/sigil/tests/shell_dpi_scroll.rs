#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

#[allow(dead_code)]
#[path = "../src/shell/event_map.rs"]
mod event_map;

use sigil::hal;
use sigil::hal::{Point, Scale, ScrollDelta};
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

#[test]
fn pixel_delta_at_scale_2x_reports_logical_pixels() {
    let delta = event_map::map_scroll_delta(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(60.0, 30.0)),
        Scale::new(2.0),
    );

    assert_eq!(delta, ScrollDelta::Pixels(Point::new(30.0, 15.0)));
}

#[test]
fn line_delta_is_not_scaled() {
    let delta = event_map::map_scroll_delta(MouseScrollDelta::LineDelta(3.0, 1.0), Scale::new(2.0));

    assert_eq!(delta, ScrollDelta::Lines(Point::new(3.0, 1.0)));
}
