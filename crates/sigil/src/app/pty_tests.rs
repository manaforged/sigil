#![allow(clippy::unwrap_used)]

use super::rows_of;

#[test]
fn vt100_clear_and_print_lands_on_first_row() {
    let mut parser = vt100::Parser::new(24, 80, 0);
    parser.process(b"\x1b[2J\x1b[Hhello");
    let rows = rows_of(&parser);
    let first = rows.first().map_or("", String::as_str);
    assert!(
        first.contains("hello"),
        "expected hello after CUP; got {rows:?}"
    );
}

#[test]
fn a_mouse_click_reports_in_the_encoding_the_program_chose() {
    use vt100::{MouseProtocolEncoding as Encoding, MouseProtocolMode as Mode};

    use super::mouse::report;
    use crate::events::{Modifiers, MouseButton};
    let none = Modifiers::empty();
    let down = super::Mouse::Down(MouseButton::Left);
    let up = super::Mouse::Up(MouseButton::Left);
    assert_eq!(report(down, (2, 4), none, Mode::None, Encoding::Sgr), None);
    assert_eq!(
        report(down, (2, 4), none, Mode::PressRelease, Encoding::Sgr).unwrap(),
        b"\x1b[<0;3;5M"
    );
    assert_eq!(
        report(up, (2, 4), none, Mode::PressRelease, Encoding::Sgr).unwrap(),
        b"\x1b[<0;3;5m"
    );
    assert_eq!(report(up, (2, 4), none, Mode::Press, Encoding::Sgr), None);
    assert_eq!(
        report(
            super::Mouse::Wheel { up: true },
            (0, 0),
            none,
            Mode::Press,
            Encoding::Default
        )
        .unwrap(),
        [0x1b, b'[', b'M', 96, 33, 33]
    );
}
