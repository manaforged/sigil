use crate::events::{Key, KeyEvent, KeyPhase, LogicalKey, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::{LayoutBuilderExt, LayoutStyle, rows};
use crate::reactive::{Signal, signal};
use crate::style::{FocusRing, NodeStyleExt, Token};

use super::virtual_list::virtual_list;

#[must_use]
pub fn terminal(lines: Signal<Vec<String>>, on_input: impl Fn(String) + 'static) -> NodeBuilder {
    let list = virtual_list(lines, |_i, line| {
        container().child(text(line.clone()).class("font-mono text-sm"))
    })
    .row_height(rows(1.0))
    .follow_end(true);
    let focused = signal(false);
    container()
        .role(Role::TextInput)
        .focusable(true)
        .focus_ring(FocusRing::new(focused))
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
    if let Some(text) = &event.text
        && !text.is_empty()
    {
        return Some(text.to_string());
    }
    match &event.key {
        LogicalKey::Named(Key::Enter) => Some("\n"),
        LogicalKey::Named(Key::Backspace) => Some("\u{7f}"),
        LogicalKey::Named(Key::Tab) => Some("\t"),
        _ => None,
    }
    .map(String::from)
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
