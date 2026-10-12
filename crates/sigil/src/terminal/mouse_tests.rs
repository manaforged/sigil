#![allow(clippy::indexing_slicing)]

use super::*;

#[test]
fn click_is_left_down_then_up_at_cell_centre() {
    let pointers: Vec<PointerEvent> = click(2, 3)
        .into_iter()
        .filter_map(|event| match event {
            ShellEvent::Pointer(pointer) => Some(pointer),
            _ => None,
        })
        .collect();
    let phases: Vec<PointerPhase> = pointers.iter().map(|pointer| pointer.phase).collect();
    assert_eq!(phases, [PointerPhase::Down, PointerPhase::Up]);
    assert_eq!(pointers[0].position, cell_point(2, 3));
}

#[test]
fn wheel_scrolls_up_from_the_same_cell() {
    let delta = match wheel(1, 1, 3.0) {
        ShellEvent::Scroll(ScrollEvent {
            delta: ScrollDelta::Lines(delta),
            ..
        }) => Some(delta),
        _ => None,
    }
    .expect("wheel is a line scroll");
    assert!((delta.y - 3.0).abs() < f32::EPSILON);
}
