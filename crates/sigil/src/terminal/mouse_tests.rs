use super::*;

#[test]
fn click_is_left_down_then_up_at_cell_centre() {
    let [down, up] = click(2, 3);
    assert!(matches!(down, ShellEvent::Pointer(_)));
    assert!(matches!(up, ShellEvent::Pointer(_)));
    if let (ShellEvent::Pointer(first), ShellEvent::Pointer(second)) = (down, up) {
        assert_eq!(first.phase, PointerPhase::Down);
        assert_eq!(second.phase, PointerPhase::Up);
        let at = cell_point(2, 3);
        assert!((first.position.x - at.x).abs() < f32::EPSILON);
        assert!((first.position.y - at.y).abs() < f32::EPSILON);
    }
}

#[test]
fn wheel_scrolls_up_from_the_same_cell() {
    let event = wheel(1, 1, 3.0);
    assert!(matches!(
        event,
        ShellEvent::Scroll(ScrollEvent {
            delta: ScrollDelta::Lines(_),
            ..
        })
    ));
    if let ShellEvent::Scroll(ScrollEvent {
        delta: ScrollDelta::Lines(delta),
        ..
    }) = event
    {
        assert!((delta.y - 3.0).abs() < f32::EPSILON);
    }
}
