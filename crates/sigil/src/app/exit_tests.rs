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

#[test]
fn an_exit_requested_outside_a_frame_ends_the_next_frame() {
    clear_exit();
    request_exit();
    let frame = std::rc::Rc::new(std::cell::Cell::new(false));
    let previous = super::swap_target(Some(std::rc::Rc::clone(&frame)));

    let exit = take_exit();

    drop(super::swap_target(previous));
    assert!(exit);
    assert!(!take_exit());
}
