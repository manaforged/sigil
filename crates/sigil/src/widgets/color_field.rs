use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, cols};
use crate::reactive::Reactive;
use crate::reactive::{Signal, derived, effect, signal};
use crate::style::{
    BORDER_WIDTH, ControlSize, Metrics, Paint, Radius, Rgba8, StyleAtom, Token, space,
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
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn invalid_when(mut self, invalid: impl crate::reactive::Reactive<bool>) -> Self {
        self.invalid = Some(invalid.into_signal());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let value = self.value;
        let buffer = signal(to_hex(value.peek()));
        effect(move || {
            let rendered = to_hex(value.get());
            buffer.set(rendered);
        });
        let (invalid, disabled) = (self.invalid, self.disabled);
        let field = text_field(buffer)
            .invalid_when(derived(move || {
                invalid.is_some_and(|flag| flag.get()) || parse_hex(&buffer.get()).is_none()
            }))
            .change(move |raw| {
                if let Some(parsed) = parse_hex(raw) {
                    value.set(parsed);
                }
            });
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

impl IntoNode for ColorFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
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
