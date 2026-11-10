use crate::events::{Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState};
use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::{LayoutStyle, rows};
use crate::reactive::{Signal, signal};
use crate::style::Token;
use crate::widgets::{Layout, Widget};

use super::virtual_list::virtual_list;

/// An embedded terminal emulator view.
#[must_use]
pub fn terminal(lines: Signal<Vec<String>>, on_input: impl Fn(String) + 'static) -> NodeBuilder {
    let list = virtual_list(lines, |_i, line| {
        container().child(text(line.clone()).class("font-mono text-sm"))
    })
    .row_height(rows(1.0))
    .follow_end(true)
    .keyboard(false);
    let focused = signal(false);
    shell(move |event| key_payload(event, false).map(&on_input).is_some())
        .focus_ring(focused)
        .focus_in(move || focused.set(true))
        .focus_out(move || focused.set(false))
        .child(list)
}

fn shell(on_key: impl Fn(&KeyEvent) -> bool + 'static) -> NodeBuilder {
    container()
        .role(Role::Terminal)
        .focusable(true)
        .layout(LayoutStyle::flex_column().fill())
        .class("font-mono")
        .class(Token::Surface.background())
        .key_any(move |event: &mut KeyEvent| {
            if on_key(event) {
                event.stop_propagation();
            }
        })
}

fn key_payload(event: &KeyEvent, application: bool) -> Option<String> {
    if event.phase != KeyPhase::Down || event.modifiers.contains(ModifiersState::META) {
        return None;
    }
    let named = match &event.key {
        LogicalKey::Named(key) => named_sequence(*key, event.modifiers, application),
        LogicalKey::Character(text) if event.modifiers.contains(ModifiersState::CONTROL) => {
            control_byte(text)
                .map(|byte| alt_prefixed(event.modifiers, char::from(byte).to_string()))
        }
        _ => None,
    };
    named.or_else(|| typed(event))
}

fn typed(event: &KeyEvent) -> Option<String> {
    if event.modifiers.contains(ModifiersState::CONTROL) {
        return None;
    }
    let text = event.text.clone().filter(|text| !text.is_empty())?;
    Some(if text.is_ascii() {
        alt_prefixed(event.modifiers, text)
    } else {
        text
    })
}

fn alt_prefixed(modifiers: ModifiersState, text: String) -> String {
    if modifiers.contains(ModifiersState::ALT) {
        format!("\u{1b}{text}")
    } else {
        text
    }
}

const fn control_byte(key: &str) -> Option<u8> {
    let [byte] = key.as_bytes() else {
        return None;
    };
    match byte.to_ascii_lowercase() {
        letter @ b'a'..=b'z' => Some(letter & 0x1f),
        b'[' => Some(0x1b),
        b'\\' => Some(0x1c),
        b']' => Some(0x1d),
        b' ' | b'@' => Some(0),
        b'^' => Some(0x1e),
        b'_' | b'/' => Some(0x1f),
        b'?' => Some(0x7f),
        _ => None,
    }
}

fn modifier_code(modifiers: ModifiersState) -> u8 {
    let bit = |key: ModifiersState, value: u8| if modifiers.contains(key) { value } else { 0 };
    1 + bit(ModifiersState::SHIFT, 1)
        + bit(ModifiersState::ALT, 2)
        + bit(ModifiersState::CONTROL, 4)
}

fn cursor(end: char, code: u8, application: bool) -> String {
    match (code, application) {
        (1, true) => format!("\u{1b}O{end}"),
        (1, false) => format!("\u{1b}[{end}"),
        _ => format!("\u{1b}[1;{code}{end}"),
    }
}

fn tilde(number: u8, code: u8) -> String {
    if code == 1 {
        format!("\u{1b}[{number}~")
    } else {
        format!("\u{1b}[{number};{code}~")
    }
}

fn edit_sequence(key: Key, modifiers: ModifiersState) -> Option<String> {
    let has = |flag: ModifiersState| modifiers.contains(flag);
    Some(match key {
        Key::Enter if has(ModifiersState::SHIFT) || has(ModifiersState::ALT) => {
            "\u{1b}\r".to_string()
        }
        Key::Enter => "\r".to_string(),
        Key::Backspace if has(ModifiersState::CONTROL) => "\u{8}".to_string(),
        Key::Backspace => alt_prefixed(modifiers, "\u{7f}".to_string()),
        Key::Tab if has(ModifiersState::CONTROL) => return None,
        Key::Tab if has(ModifiersState::SHIFT) => "\u{1b}[Z".to_string(),
        Key::Tab => "\t".to_string(),
        Key::Escape => "\u{1b}".to_string(),
        _ => return None,
    })
}

const fn cursor_end(key: Key) -> Option<(char, bool)> {
    Some(match key {
        Key::ArrowUp => ('A', false),
        Key::ArrowDown => ('B', false),
        Key::ArrowRight => ('C', false),
        Key::ArrowLeft => ('D', false),
        Key::Home => ('H', false),
        Key::End => ('F', false),
        Key::F1 => ('P', true),
        Key::F2 => ('Q', true),
        Key::F3 => ('R', true),
        Key::F4 => ('S', true),
        _ => return None,
    })
}

const fn tilde_number(key: Key) -> Option<u8> {
    Some(match key {
        Key::Insert => 2,
        Key::Delete => 3,
        Key::PageUp => 5,
        Key::PageDown => 6,
        Key::F5 => 15,
        Key::F6 => 17,
        Key::F7 => 18,
        Key::F8 => 19,
        Key::F9 => 20,
        Key::F10 => 21,
        Key::F11 => 23,
        Key::F12 => 24,
        _ => return None,
    })
}

fn named_sequence(key: Key, modifiers: ModifiersState, application: bool) -> Option<String> {
    let code = modifier_code(modifiers);
    if let Some((end, always)) = cursor_end(key) {
        return Some(cursor(end, code, always || application));
    }
    tilde_number(key)
        .map(|number| tilde(number, code))
        .or_else(|| edit_sequence(key, modifiers))
}

#[cfg(feature = "pty")]
#[path = "terminal_session.rs"]
mod session;

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
