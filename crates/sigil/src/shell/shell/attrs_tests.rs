use crate::app::WindowOpts;
use winit::window::WindowLevel;

use crate::shell::WinitWindowHandle;

use super::*;

fn spec(opts: WindowOpts) -> PendingWindow {
    PendingWindow {
        handle: WinitWindowHandle::from_raw(0),
        title: "test".to_owned(),
        size: PendingWindowSize::Logical(LogicalSize::new(100.0, 100.0)),
        opts,
    }
}

#[test]
fn bottom_center_centers_and_clears_margin() {
    let (x, y) = anchor_origin(
        Anchor::BottomCenter { margin: 64.0 },
        (0, 0),
        (2560, 1440),
        (200, 48),
        1.0,
    );
    assert_eq!(x, (2560 - 200) / 2);
    assert_eq!(y, 1440 - 48 - 64);
}

#[test]
fn bottom_center_scales_margin_by_monitor_factor() {
    let (_, y) = anchor_origin(
        Anchor::BottomCenter { margin: 64.0 },
        (0, 0),
        (5120, 2880),
        (400, 96),
        2.0,
    );
    assert_eq!(y, 2880 - 96 - 128);
}

#[test]
fn bottom_center_respects_monitor_origin() {
    let (x, y) = anchor_origin(
        Anchor::BottomCenter { margin: 10.0 },
        (-1920, 200),
        (1920, 1080),
        (100, 40),
        1.0,
    );
    assert_eq!(x, -1920 + (1920 - 100) / 2);
    assert_eq!(y, 200 + 1080 - 40 - 10);
}

#[test]
fn default_attrs_keep_os_decorations() {
    let attrs = WinitShell::base_attrs(&spec(WindowOpts::default()));

    assert!(attrs.decorations);
    assert!(attrs.resizable);
    assert!(attrs.active);
    assert!(!attrs.transparent);
    assert_eq!(attrs.window_level, WindowLevel::Normal);
}

#[test]
fn frameless_undecorates_but_stays_a_normal_window() {
    let attrs = WinitShell::base_attrs(&spec(WindowOpts {
        frameless: true,
        ..WindowOpts::default()
    }));

    assert!(!attrs.decorations);
    assert!(attrs.resizable);
    assert!(attrs.active);
    assert_eq!(attrs.window_level, WindowLevel::Normal);
    #[cfg(target_os = "macos")]
    assert!(attrs.transparent);
    #[cfg(not(target_os = "macos"))]
    assert!(!attrs.transparent);
}

#[test]
fn overlay_wins_when_both_flags_are_set() {
    let attrs = WinitShell::base_attrs(&spec(WindowOpts {
        overlay: true,
        frameless: true,
        ..WindowOpts::default()
    }));

    assert!(!attrs.decorations);
    assert!(attrs.transparent);
    assert!(!attrs.active);
    assert_eq!(attrs.window_level, WindowLevel::AlwaysOnTop);
}
