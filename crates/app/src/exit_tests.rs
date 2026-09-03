use super::{clear_exit, request_exit, take_exit};

#[test]
fn request_exit_latches_once() {
    clear_exit();
    assert!(!take_exit());
    request_exit();
    request_exit();
    assert!(take_exit());
    assert!(!take_exit());
}
