#![allow(clippy::redundant_pub_crate)]

use std::sync::Arc;

use crate::geometry::{Point, Scale};
use winit::window::{Window, WindowId};

use crate::shell::WinitWindowHandle;
use crate::shell::event_map::point_from_f64;

pub(crate) fn logical_point_from_f64(x: f64, y: f64, scale: Scale) -> Point {
    scale.point_to_logical(point_from_f64(x, y))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
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
#[path = "dpi_tests.rs"]
mod tests;
