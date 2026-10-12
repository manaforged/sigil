use super::*;

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
