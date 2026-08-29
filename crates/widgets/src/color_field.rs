use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, effect, signal};
use sigil_style::{NodeStyleExt, Rgba8, Style};

use super::text_field::text_field;

#[must_use]
pub fn color_field(value: Signal<Rgba8>) -> NodeBuilder {
    let buffer = signal(to_hex(value.peek()));
    let buf_for_value = buffer;
    effect(move || {
        let rendered = to_hex(value.get());
        if buf_for_value.peek().as_str() != rendered {
            buf_for_value.set(rendered);
        }
    });
    let field = text_field(buffer).change(move |raw| {
        let Some(parsed) = parse_hex(raw) else {
            return;
        };
        if value.peek() != parsed {
            value.set(parsed);
        }
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .gap(8.0),
        )
        .child(swatch(value))
        .child(field)
}

fn swatch(value: Signal<Rgba8>) -> NodeBuilder {
    container().children_reactive(move || {
        vec![
            container()
                .layout(LayoutStyle::block().width(28.0).height(28.0))
                .class("rounded-md border border-gray-6")
                .style(Style::new().bg(value.get())),
        ]
    })
}

#[must_use]
pub fn parse_hex(input: &str) -> Option<Rgba8> {
    let hex = input.trim().strip_prefix('#')?;
    match hex.len() {
        3 => {
            let mut nibbles = hex.bytes();
            let r = nibble(nibbles.next()?)?;
            let g = nibble(nibbles.next()?)?;
            let b = nibble(nibbles.next()?)?;
            Some(Rgba8 {
                r: r * 17,
                g: g * 17,
                b: b * 17,
                a: 255,
            })
        }
        6 => {
            let r = byte(&hex[0..2])?;
            let g = byte(&hex[2..4])?;
            let b = byte(&hex[4..6])?;
            Some(Rgba8 { r, g, b, a: 255 })
        }
        _ => None,
    }
}

#[must_use]
pub fn to_hex(color: Rgba8) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

const fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn byte(pair: &str) -> Option<u8> {
    u8::from_str_radix(pair, 16).ok()
}

#[cfg(test)]
#[path = "color_field_tests.rs"]
mod tests;
