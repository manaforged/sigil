//! DPI conversion helpers for winit event translation.

#![allow(clippy::redundant_pub_crate)]

use std::sync::Arc;

use hal::{Point, Scale};
use winit::window::{Window, WindowId};

use crate::WinitWindowHandle;
use crate::event_map::point_from_f64;

pub(crate) fn logical_point_from_f64(x: f64, y: f64, scale: Scale) -> Point {
    scale.point_to_logical(point_from_f64(x, y))
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn scale_from_f64(scale: f64) -> Scale {
    Scale::new(scale as f32)
}

pub(crate) fn scale_for_window_id(
    windows: &[(WinitWindowHandle, Arc<Window>)],
    window_scales: &[(WinitWindowHandle, Scale)],
    window_id: WindowId,
) -> Scale {
    handle_for_window_id(windows, window_id)
        .and_then(|handle| {
            window_scales
                .iter()
                .find_map(|(stored, scale)| (*stored == handle).then_some(*scale))
        })
        .unwrap_or(Scale::ONE)
}

pub(crate) fn set_scale_for_window_id(
    windows: &[(WinitWindowHandle, Arc<Window>)],
    window_scales: &mut Vec<(WinitWindowHandle, Scale)>,
    window_id: WindowId,
    scale: Scale,
) {
    let Some(handle) = handle_for_window_id(windows, window_id) else {
        return;
    };
    if let Some((_, stored)) = window_scales
        .iter_mut()
        .find(|(stored, _)| *stored == handle)
    {
        *stored = scale;
    } else {
        window_scales.push((handle, scale));
    }
}

fn handle_for_window_id(
    windows: &[(WinitWindowHandle, Arc<Window>)],
    window_id: WindowId,
) -> Option<WinitWindowHandle> {
    windows
        .iter()
        .find_map(|(handle, window)| (window.id() == window_id).then_some(*handle))
}

#[cfg(test)]
mod tests {
    use hal::{Point, Scale, WindowEvent};

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
}
