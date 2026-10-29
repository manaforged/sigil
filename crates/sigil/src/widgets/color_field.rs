use super::field::{FieldParser, ParsedField};
use super::layout_ext::WidgetLayoutExt;
use super::shared::disabled_look;
use crate::kernel::{NodeBuilder, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, cols};
use crate::reactive::Reactive;
use crate::reactive::Signal;
use crate::style::locale::Locale;
use crate::style::{
    BORDER_WIDTH, ControlSize, Metrics, Paint, Radius, Rgba8, StyleAtom, Token, space,
};

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
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn invalid_when(mut self, invalid: impl Reactive<bool>) -> Self {
        self.invalid = Some(invalid.into_signal());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let (value, disabled) = (self.value, self.disabled);
        let mut field = ParsedField::new(value, HexParser);
        if let Some(invalid) = self.invalid {
            field = field.invalid_when(invalid);
        }
        let field = field.text_field();
        let field = match disabled {
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
            .child(disabled_look(swatch(value), disabled))
            .child(field)
    }
}

builder_impls!([] ColorFieldBuilder: plain_node);

struct HexParser;

impl FieldParser for HexParser {
    type Value = Rgba8;

    fn parse(&self, raw: &str, _locale: &Locale) -> Option<Rgba8> {
        parse_hex(raw)
    }

    fn render(&self, value: &Rgba8, _locale: &Locale) -> String {
        to_hex(*value)
    }
}

fn swatch(value: Signal<Rgba8>) -> NodeBuilder {
    let extent = Metrics::current().control_height(ControlSize::Default);
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
                .bg(value.get()),
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
    let digits = hex.bytes().map(nibble).collect::<Option<Vec<u8>>>()?;
    let (r, g, b) = match *digits.as_slice() {
        [r, g, b] => (r * 17, g * 17, b * 17),
        [r1, r0, g1, g0, b1, b0] => (r1 * 16 + r0, g1 * 16 + g0, b1 * 16 + b0),
        _ => return None,
    };
    Some(Rgba8 { r, g, b, a: 255 })
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

#[cfg(test)]
#[path = "color_field_tests.rs"]
mod tests;
