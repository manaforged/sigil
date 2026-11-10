use super::*;

#[test]
fn install_after_clear_queues_again() {
    PENDING.with(|slot| *slot.borrow_mut() = None);
    Tray::new().tooltip("one").install().expect("install");
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
    Tray::new().tooltip("two").install().expect("install");
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn a_bad_icon_is_an_error_and_queues_nothing() {
    PENDING.with(|slot| *slot.borrow_mut() = None);
    let error = Tray::new().icon(vec![0; 3], 1, 1).install();
    assert_eq!(error.expect_err("bad icon").kind(), ErrorKind::Other);
    assert!(!has_pending());
    Tray::new()
        .icon(vec![0; 4], 1, 1)
        .install()
        .expect("good icon");
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
}
