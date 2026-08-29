use sigil_hal::Rect;
use winit::dpi::{LogicalSize as WinitLogicalSize, PhysicalPosition};

use crate::WinitWindowHandle;

use super::WinitShell;

impl WinitShell {
    pub fn place_overlay(
        &self,
        child: WinitWindowHandle,
        parent: WinitWindowHandle,
        hole: Option<Rect>,
    ) {
        let Some(child_w) = self.window_for(child) else {
            return;
        };
        let Some(rect) = hole else {
            child_w.set_visible(false);
            return;
        };
        let Some(parent_w) = self.window_for(parent) else {
            return;
        };
        let Ok(origin) = parent_w.inner_position() else {
            return;
        };
        let (x, y) = overlay_origin(
            (origin.x, origin.y),
            (rect.origin.x, rect.origin.y),
            parent_w.scale_factor(),
        );
        child_w.set_outer_position(PhysicalPosition::new(x, y));
        let _applied = child_w.request_inner_size(WinitLogicalSize::new(
            f64::from(rect.width),
            f64::from(rect.height),
        ));
        child_w.set_visible(true);
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
pub(crate) fn overlay_origin(parent: (i32, i32), origin: (f32, f32), scale: f64) -> (i32, i32) {
    (
        parent.0 + (f64::from(origin.0) * scale).round() as i32,
        parent.1 + (f64::from(origin.1) * scale).round() as i32,
    )
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
