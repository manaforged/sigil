use std::rc::Rc;

use sigil_events::prelude::{Key, KeyEvent, KeyPhase, LogicalKey, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::NodeStyleExt;

use super::virtual_list::virtual_list;

#[must_use]
pub fn terminal(lines: Signal<Vec<String>>, on_input: impl Fn(String) + 'static) -> NodeBuilder {
    let on_input = Rc::new(on_input);
    let list = virtual_list(lines, |_i, line| {
        container().child(text(line.clone()).class("font-mono text-sm"))
    })
    .auto_pin_to_bottom(true);
    container()
        .role(Role::TextInput)
        .focusable(true)
        .layout(LayoutStyle::flex_column().fill())
        .class("bg-surface font-mono")
        .key_any({
            let on_input = Rc::clone(&on_input);
            move |event: &mut KeyEvent| {
                if let Some(payload) = key_payload(event) {
                    on_input(payload);
                    event.stop_propagation();
                }
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
        LogicalKey::Named(Key::Enter) => Some(String::from("\n")),
        LogicalKey::Named(Key::Backspace) => Some(String::from("\u{7f}")),
        LogicalKey::Named(Key::Tab) => Some(String::from("\t")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
