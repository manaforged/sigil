use vt100::{MouseProtocolEncoding, MouseProtocolMode};

use crate::events::{Modifiers, MouseButton};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mouse {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Move,
    Wheel { up: bool },
}

const fn button_code(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

const fn wanted(mouse: Mouse, mode: MouseProtocolMode) -> bool {
    match (mouse, mode) {
        (_, MouseProtocolMode::None) | (Mouse::Up(_), MouseProtocolMode::Press) => false,
        (Mouse::Down(_) | Mouse::Up(_) | Mouse::Wheel { .. }, _)
        | (Mouse::Drag(_), MouseProtocolMode::ButtonMotion | MouseProtocolMode::AnyMotion)
        | (Mouse::Move, MouseProtocolMode::AnyMotion) => true,
        _ => false,
    }
}

const fn code(mouse: Mouse) -> Option<u16> {
    match mouse {
        Mouse::Down(button) | Mouse::Up(button) => button_code(button),
        Mouse::Drag(button) => match button_code(button) {
            Some(code) => Some(code + 32),
            None => None,
        },
        Mouse::Move => Some(35),
        Mouse::Wheel { up: true } => Some(64),
        Mouse::Wheel { up: false } => Some(65),
    }
}

fn held(modifiers: Modifiers) -> u16 {
    let bit = |key: Modifiers, value: u16| if modifiers.contains(key) { value } else { 0 };
    bit(Modifiers::SHIFT, 4) + bit(Modifiers::ALT, 8) + bit(Modifiers::CTRL, 16)
}

/// The bytes a program in `mode` expects for `mouse` at the zero-based `(col, row)` cell.
pub(super) fn report(
    mouse: Mouse,
    at: (u16, u16),
    modifiers: Modifiers,
    mode: MouseProtocolMode,
    encoding: MouseProtocolEncoding,
) -> Option<Vec<u8>> {
    if !wanted(mouse, mode) {
        return None;
    }
    let value = code(mouse)? + held(modifiers);
    let (col, row) = (at.0 + 1, at.1 + 1);
    let up = matches!(mouse, Mouse::Up(_));
    Some(match encoding {
        MouseProtocolEncoding::Sgr => {
            let end = if up { 'm' } else { 'M' };
            format!("\u{1b}[<{value};{col};{row}{end}").into_bytes()
        }
        MouseProtocolEncoding::Utf8 => {
            let value = if up { 3 } else { value };
            let mut out = String::from("\u{1b}[M");
            for number in [value + 32, col + 32, row + 32] {
                out.push(char::from_u32(u32::from(number))?);
            }
            out.into_bytes()
        }
        MouseProtocolEncoding::Default => {
            let value = if up { 3 } else { value };
            let byte = |number: u16| u8::try_from(number + 32).ok();
            vec![0x1b, b'[', b'M', byte(value)?, byte(col)?, byte(row)?]
        }
    })
}
