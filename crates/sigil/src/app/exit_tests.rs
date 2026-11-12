use super::{clear_exit, exit, take_exit};

#[test]
fn request_exit_latches_once() {
    clear_exit();
    assert!(!take_exit());
    exit();
    exit();
    assert!(take_exit());
    assert!(!take_exit());
}
