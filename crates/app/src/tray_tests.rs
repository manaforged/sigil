use super::*;

#[test]
fn set_tray_marks_pending() {
    assert!(!has_pending() || PENDING.with(|s| s.borrow().is_some()));
    set_tray(TrayIconBuilder::new().with_tooltip("probe"));
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn set_tray_after_clear_queues_again() {
    PENDING.with(|slot| *slot.borrow_mut() = None);
    set_tray(TrayIconBuilder::new().with_tooltip("one"));
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
    set_tray(TrayIconBuilder::new().with_tooltip("two"));
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
}
