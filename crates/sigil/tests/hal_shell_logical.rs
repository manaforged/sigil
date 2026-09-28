#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[path = "events_support/mock_shell.rs"]
mod mock_shell;

use mock_shell::MockShell;
use sigil::hal::{LogicalSize, PhysicalSize, Shell};

#[test]
fn default_create_window_logical_uses_scale_one_physical_size() {
    let mut shell = MockShell::default();

    let handle = shell
        .create_window_logical("test", LogicalSize::new(720.0, 480.0))
        .expect("default logical create should delegate");

    assert_eq!(handle, 0);
    assert_eq!(
        shell.created,
        vec![("test".to_owned(), PhysicalSize::new(720, 480))]
    );
}

#[test]
fn default_create_window_with_ignores_opts_and_delegates() {
    let mut shell = MockShell::default();

    let handle = shell
        .create_window_with(
            "pill",
            LogicalSize::new(200.0, 48.0),
            sigil::hal::WindowOpts {
                overlay: true,
                frameless: false,
                anchor: Some(sigil::hal::Anchor::BottomCenter { margin: 64.0 }),
                user_zoom: None,
                ..sigil::hal::WindowOpts::default()
            },
        )
        .expect("default opts create should delegate");

    assert_eq!(handle, 0);
    assert_eq!(
        shell.created,
        vec![("pill".to_owned(), PhysicalSize::new(200, 48))]
    );
}
