use winit::window::Window;

#[cfg(target_os = "macos")]
const CONTROLS_LEFT: f64 = 16.0;
#[cfg(target_os = "macos")]
const CONTROLS_GAP: f64 = 20.0;

/// Centres the macOS window controls in an inset title bar `bar` points tall.
#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "AppKit view lookups are unsafe in objc2; the views belong to this live window"
)]
pub(super) fn centre_controls(window: &Window, bar: f32) {
    use objc2_app_kit::{NSView, NSWindowButton};
    use objc2_foundation::{NSPoint, NSRect};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(kit) = handle.as_raw() else {
        return;
    };
    // SAFETY: winit hands out the live NSView of this window, and events run on the main thread.
    let view: &NSView = unsafe { kit.ns_view.cast().as_ref() };
    let Some(ns_window) = view.window() else {
        return;
    };
    let bar = f64::from(bar);
    let buttons = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ];
    let Some(close) = ns_window.standardWindowButton(buttons[0]) else {
        return;
    };
    // SAFETY: the close button and its title bar view are live subviews of this window.
    let Some(container) =
        (unsafe { close.superview() }).and_then(|bar_view| unsafe { bar_view.superview() })
    else {
        return;
    };
    let height = ns_window.frame().size.height;
    let mut frame: NSRect = container.frame();
    if (frame.origin.y - (height - bar)).abs() > 0.5 || (frame.size.height - bar).abs() > 0.5 {
        frame.origin.y = height - bar;
        frame.size.height = bar;
        container.setFrame(frame);
    }
    for (index, kind) in buttons.into_iter().enumerate() {
        let Some(button) = ns_window.standardWindowButton(kind) else {
            continue;
        };
        let now = button.frame();
        #[allow(clippy::cast_precision_loss, reason = "three buttons")]
        let x = (index as f64).mul_add(CONTROLS_GAP, CONTROLS_LEFT);
        let y = (bar - now.size.height) / 2.0;
        if (now.origin.x - x).abs() > 0.5 || (now.origin.y - y).abs() > 0.5 {
            button.setFrameOrigin(NSPoint::new(x, y));
        }
    }
}

/// Leaves the window controls where the platform puts them.
#[cfg(not(target_os = "macos"))]
pub(super) const fn centre_controls(_window: &Window, _bar: f32) {}
