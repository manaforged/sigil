use crate::events::{Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState};
use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::{LayoutStyle, rows};
use crate::reactive::{Signal, signal};
use crate::style::Token;
use crate::widgets::Widget;

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
    container()
        .role(Role::Terminal)
        .focusable(true)
        .focus_ring(focused)
        .focus_in(move || focused.set(true))
        .focus_out(move || focused.set(false))
        .layout(LayoutStyle::flex_column().fill())
        .class("font-mono")
        .class(Token::Surface.bg())
        .key_any(move |event: &mut KeyEvent| {
            if let Some(payload) = key_payload(event) {
                on_input(payload);
                event.stop_propagation();
            }
        })
        .child(list)
}

fn key_payload(event: &KeyEvent) -> Option<String> {
    if event.phase != KeyPhase::Down {
        return None;
    }
    if event.modifiers.contains(ModifiersState::CONTROL)
        && let LogicalKey::Character(letter) = &event.key
        && let [byte @ b'a'..=b'z'] = letter.to_ascii_lowercase().as_bytes()
    {
        return Some(char::from(byte & 0x1f).to_string());
    }
    if let LogicalKey::Named(key) = &event.key
        && let Some(sequence) = named_sequence(*key)
    {
        return Some(sequence.to_string());
    }
    event.text.clone().filter(|text| !text.is_empty())
}

fn named_sequence(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Enter => "\r",
        Key::Backspace => "\u{7f}",
        Key::Tab => "\t",
        Key::Escape => "\u{1b}",
        Key::ArrowUp => "\u{1b}[A",
        Key::ArrowDown => "\u{1b}[B",
        Key::ArrowRight => "\u{1b}[C",
        Key::ArrowLeft => "\u{1b}[D",
        Key::Home => "\u{1b}[H",
        Key::End => "\u{1b}[F",
        Key::Delete => "\u{1b}[3~",
        _ => return None,
    })
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
