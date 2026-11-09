use super::*;

#[test]
fn install_after_clear_queues_again() {
    NATIVE.with(Native::take);
    Tray::new().tooltip("one").install().expect("install");
    assert!(has_pending());
    NATIVE.with(Native::take);
    Tray::new().tooltip("two").install().expect("install");
    assert!(has_pending());
    NATIVE.with(Native::take);
}

#[test]
fn a_bad_icon_is_an_error_and_queues_nothing() {
    NATIVE.with(Native::take);
    let error = Tray::new().icon(vec![0; 3], 1, 1).install();
    assert_eq!(error.expect_err("bad icon").kind(), ErrorKind::Other);
    assert!(!has_pending());
    Tray::new()
        .icon(vec![0; 4], 1, 1)
        .install()
        .expect("good icon");
    assert!(has_pending());
    NATIVE.with(Native::take);
}
