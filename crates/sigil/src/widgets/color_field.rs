use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::prelude::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, cols};
use crate::layout::snap::CellForm;
use crate::reactive::{Signal, derived, effect, signal};
use crate::style::{
    BORDER_WIDTH, Metrics, NodeStyleExt, Radius, Rgba8, Size, Style, StyleAtom, Token, space,
};

use super::layout_ext::WidgetLayoutExt;
use super::shared::disabled_look;
use super::text_field::text_field;

pub struct ColorFieldBuilder {
    value: Signal<Rgba8>,
    disabled: Option<Signal<bool>>,
    invalid: Option<Signal<bool>>,
}

#[must_use]
pub const fn color_field(value: Signal<Rgba8>) -> ColorFieldBuilder {
    ColorFieldBuilder {
        value,
        disabled: None,
        invalid: None,
    }
}

impl ColorFieldBuilder {
    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    #[must_use]
    pub const fn invalid_when(mut self, invalid: Signal<bool>) -> Self {
        self.invalid = Some(invalid);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let value = self.value;
        let buffer = signal(to_hex(value.peek()));
        effect(move || {
            let rendered = to_hex(value.get());
            if buffer.peek().as_str() != rendered {
                buffer.set(rendered);
            }
        });
        let invalid = self.invalid;
        let field = text_field(buffer)
            .invalid_when(derived(move || {
                invalid.is_some_and(|flag| flag.get()) || parse_hex(&buffer.get()).is_none()
            }))
            .change(move |raw| {
                let Some(parsed) = parse_hex(raw) else {
                    return;
                };
                if value.peek() != parsed {
                    value.set(parsed);
                }
            });
        let field = match self.disabled {
            Some(disabled) => field.disabled(disabled),
            None => field,
        }
        .layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .min_size(0.0, Dimension::Auto)
                .align_items(AlignItems::Center),
        );
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .align_items(AlignItems::Center)
                    .gap(space(2)),
            )
            .child(disabled_look(swatch(value), self.disabled))
            .child(field)
    }
}

impl IntoNode for ColorFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

fn swatch(value: Signal<Rgba8>) -> NodeBuilder {
    let extent = Metrics::current().control_height(Size::Default);
    let (width, chrome) = CellForm::new(
        (Dimension::Px(extent), SWATCH_CHROME),
        (cols(2.0).into(), NO_CHROME),
    )
    .pick();
    container().children_reactive(move || {
        vec![
            container()
                .layout(LayoutStyle::block().width(width).height(extent))
                .class(chrome)
                .style(Style::new().bg(value.get())),
        ]
    })
}

static SWATCH_CHROME: &[StyleAtom] = &[
    StyleAtom::Radius(Radius::Md),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Gray6.border(),
];
static NO_CHROME: &[StyleAtom] = &[];

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
