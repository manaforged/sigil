use super::*;

#[test]
fn set_tray_marks_pending() {
    // Registering a builder marks a tray pending; building it needs a
    // real desktop and is exercised on device, not here.
    assert!(!has_pending() || PENDING.with(|s| s.borrow().is_some()));
    set_tray(TrayIconBuilder::new().with_tooltip("probe"));
    assert!(has_pending());
    // Clear so the thread-local does not leak into sibling tests.
    PENDING.with(|slot| *slot.borrow_mut() = None);
}
