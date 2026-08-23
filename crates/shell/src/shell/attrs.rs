//! Window-attribute construction and pending-window materialisation on
//! [`super::WinitShell`].
//!
//! Split out of `shell.rs` to keep the lifecycle / Shell-impl portions
//! readable under the 400-LoC hygiene cap. These functions translate a
//! [`PendingWindow`] spec into winit [`WindowAttributes`] (the overlay and
//! frameless bundles live here) and create the OS window once the event
//! loop is alive.

use std::sync::Arc;

use sigil_hal::{Anchor, LogicalSize};
use winit::{
    dpi::{
        LogicalSize as WinitLogicalSize, PhysicalPosition as WinitPhysicalPosition,
        PhysicalSize as WinitPhysicalSize, Size as WinitSize,
    },
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowLevel},
};

use crate::dpi::scale_from_f64;

use super::{PendingWindow, PendingWindowSize, WinitShell};

impl WinitShell {
    /// Convert a logical size to physical pixels with the monitor's
    /// scale factor, matching winit's own `WinitSize::Logical` handling.
    fn logical_to_physical_px(size: LogicalSize, scale: f64) -> (u32, u32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        (
            (f64::from(size.width) * scale).round() as u32,
            (f64::from(size.height) * scale).round() as u32,
        )
    }

    /// Build the non-positional winit attributes for a pending spec.
    /// The overlay opt bundles undecorated, transparent, always-on-top,
    /// and inactive (plus shadowless on macOS); frameless undecorates
    /// but keeps normal level/activation/resizability and (on macOS)
    /// the shadow, so the app can draw its own chrome over an ordinary
    /// window. Overlay wins when both flags are set. Windows start
    /// hidden until the AccessKit adapter is attached —
    /// `accesskit_winit::Adapter` panics if constructed against an
    /// already-visible window.
    fn base_attrs(spec: &PendingWindow) -> WindowAttributes {
        let size = match spec.size {
            PendingWindowSize::Physical(size) => {
                WinitSize::Physical(WinitPhysicalSize::new(size.width, size.height))
            }
            PendingWindowSize::Logical(size) => WinitSize::Logical(WinitLogicalSize::new(
                f64::from(size.width),
                f64::from(size.height),
            )),
        };
        let mut attrs = WindowAttributes::default()
            .with_title(spec.title.clone())
            .with_inner_size(size)
            .with_visible(false);
        if spec.opts.overlay {
            attrs = attrs
                .with_decorations(false)
                .with_transparent(true)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_active(false);
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::WindowAttributesExtMacOS;
                attrs = attrs.with_has_shadow(false);
            }
        } else if spec.opts.frameless {
            attrs = attrs.with_decorations(false);
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::WindowAttributesExtMacOS;
                attrs = attrs.with_transparent(true).with_has_shadow(true);
            }
        }
        attrs
    }

    /// Build the winit attributes for a pending spec — [`Self::base_attrs`]
    /// plus anchor placement, which resolves against the primary monitor
    /// when one is discoverable; otherwise the OS default position stands.
    fn window_attrs(spec: &PendingWindow, active: &ActiveEventLoop) -> WindowAttributes {
        let mut attrs = Self::base_attrs(spec);
        if let Some(anchor) = spec.opts.anchor
            && let Some(monitor) = active.primary_monitor()
        {
            let scale = monitor.scale_factor();
            let win = match spec.size {
                PendingWindowSize::Physical(s) => (s.width, s.height),
                PendingWindowSize::Logical(s) => Self::logical_to_physical_px(s, scale),
            };
            let pos = monitor.position();
            let msize = monitor.size();
            let (x, y) = anchor_origin(
                anchor,
                (pos.x, pos.y),
                (msize.width, msize.height),
                win,
                scale,
            );
            attrs = attrs.with_position(WinitPhysicalPosition::new(x, y));
        }
        attrs
    }

    pub(crate) fn materialise_pending(&mut self, active: &ActiveEventLoop) {
        if self.pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending);
        for spec in pending {
            let attrs = Self::window_attrs(&spec, active);
            match active.create_window(attrs) {
                Ok(window) => {
                    if spec.opts.overlay {
                        // Click-through; failures are cosmetic, not fatal.
                        let _ = window.set_cursor_hittest(false);
                    }
                    self.install_a11y_adapter(active, spec.handle, &window);
                    let zoom = spec.opts.user_zoom.unwrap_or(1.0);
                    self.window_zooms.push((spec.handle, zoom));
                    self.window_scales.push((
                        spec.handle,
                        scale_from_f64(window.scale_factor() * f64::from(zoom)),
                    ));
                    self.windows.push((spec.handle, Arc::new(window)));
                }
                // Recorded for `take_failed_windows`; callers surface
                // the error out-of-band since the shell cannot panic.
                Err(err) => self.failed.push((spec.handle, err.to_string())),
            }
        }
    }
}

/// Resolve an [`Anchor`] to a window origin in physical pixels.
/// `monitor_pos`/`monitor_size` and `window_size` are physical pixels;
/// the anchor's margin is logical and scaled by the monitor's factor.
/// Pure so it is testable without a live monitor.
fn anchor_origin(
    anchor: Anchor,
    monitor_pos: (i32, i32),
    monitor_size: (u32, u32),
    window_size: (u32, u32),
    scale: f64,
) -> (i32, i32) {
    match anchor {
        Anchor::BottomCenter { margin } => {
            #[allow(clippy::cast_possible_truncation)]
            let margin_px = (f64::from(margin) * scale).round() as i32;
            #[allow(clippy::cast_possible_wrap)]
            let x = monitor_pos.0 + (monitor_size.0 as i32 - window_size.0 as i32) / 2;
            #[allow(clippy::cast_possible_wrap)]
            let y = monitor_pos.1 + monitor_size.1 as i32 - window_size.1 as i32 - margin_px;
            (x, y)
        }
    }
}

#[cfg(test)]
#[path = "attrs_tests.rs"]
mod tests;
