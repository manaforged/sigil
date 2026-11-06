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
    shell(on_input).child(list)
}

fn shell(on_input: impl Fn(String) + 'static) -> NodeBuilder {
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
}

#[cfg(feature = "pty")]
impl crate::app::Session {
    /// Builds a view of this terminal that fills its space, sizes the terminal to it, and takes keys, paste and the wheel.
    #[must_use]
    pub fn view(self: &std::rc::Rc<Self>) -> NodeBuilder {
        use std::rc::Rc;

        let list = virtual_list(self.rows, |_i, row: &crate::app::pty::Row| {
            container().child(text(row.text.clone()).runs(row.runs.clone()))
        })
        .row_height(rows(1.0))
        .follow_end(true)
        .keyboard(false);
        let size = signal((0.0_f32, 0.0_f32));
        let sized = Rc::clone(self);
        crate::reactive::effect_named("sigil::terminal::size", move || {
            let (width, height) = size.get();
            let (cell_width, cell_height) = crate::layout::mono_cell();
            if width > 0.0 && height > 0.0 {
                drop(sized.resize(cells(width, cell_width), cells(height, cell_height)));
            }
        });
        let (keys, pasted, wheel) = (Rc::clone(self), Rc::clone(self), Rc::clone(self));
        shell(move |bytes| drop(keys.write(bytes.as_bytes())))
            .child(list)
            .size_reactive(size)
            .on_paste(move |text| pasted.paste(text).is_ok())
            .on_scroll(move |event| {
                let count = match event.delta {
                    crate::events::ScrollDelta::Lines(point) => point.y,
                    crate::events::ScrollDelta::Pixels(point) => {
                        point.y / crate::layout::mono_cell().1.max(1.0)
                    }
                };
                let count = count.round() as i32;
                if count != 0 {
                    drop(wheel.scroll(count));
                    event.stop_propagation();
                }
            })
    }
}

#[cfg(feature = "pty")]
fn cells(span: f32, cell: f32) -> u16 {
    (span / cell.max(1.0)).floor().clamp(1.0, 999.0) as u16
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
