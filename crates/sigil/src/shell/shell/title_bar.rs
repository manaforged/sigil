use winit::window::Window;

const CONTROLS_LEFT: f64 = 16.0;
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
    let Some(container) = (unsafe { close.superview() })
        .and_then(|bar_view| unsafe { bar_view.superview() })
    else {
        return;
    };
    let height = ns_window.frame().size.height;
    let mut frame: NSRect = container.frame();
    frame.origin.y = height - bar;
    frame.size.height = bar;
    container.setFrame(frame);
    for (index, kind) in buttons.into_iter().enumerate() {
        let Some(button) = ns_window.standardWindowButton(kind) else {
            continue;
        };
        let size = button.frame().size;
        #[allow(clippy::cast_precision_loss, reason = "three buttons")]
        let x = CONTROLS_LEFT + index as f64 * CONTROLS_GAP;
        button.setFrameOrigin(NSPoint::new(x, (bar - size.height) / 2.0));
    }
}

/// Leaves the window controls where the platform puts them.
#[cfg(not(target_os = "macos"))]
pub(super) fn centre_controls(_window: &Window, _bar: f32) {}
