use super::*;

#[test]
fn clean_frame_writes_no_terminal_bytes() {
    let mut screen = Screen::new(1, 1);
    screen.flush(&mut Vec::new()).expect("prime screen");
    let mut bytes = Vec::new();
    paint_frame(&mut bytes, &mut screen, false).expect("paint");
    assert!(bytes.is_empty(), "idle frame wrote {bytes:?}");
}
