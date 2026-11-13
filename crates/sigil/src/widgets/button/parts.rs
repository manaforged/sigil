use crate::animation::Transition;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Space};
use crate::node::{NodeBuilder, TextBuilder, TextContent, text};
use crate::style::{
    BORDER_WIDTH, Color, ColorValue, Elevation, Metrics, Paint, Radius, States, Token, Tone,
};
use crate::widgets::Widget;

#[derive(Clone, Copy)]
pub(super) enum ButtonWidth {
    Hug,
    Fill,
    Fixed(f32),
}

pub(super) fn button_layout(width: ButtonWidth, height: f32, padding_x: f32) -> LayoutStyle {
    let layout = LayoutStyle::flex_row()
        .height(height)
        .padding_x(Space::raw(padding_x, "control padding"))
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center);
    match width {
        ButtonWidth::Hug => layout.shrink(0.0),
        ButtonWidth::Fill => layout,
        ButtonWidth::Fixed(px) => layout.width(px).shrink(0.0),
    }
}

#[derive(Clone, Copy)]
struct ButtonPalette {
    border: ColorValue,
    rest_bg: Option<Token>,
    rest_fg: ColorValue,
    hover_bg: Token,
    pressed_bg: Token,
    disabled_bg: Option<Token>,
    shadow: Option<Elevation>,
}

impl ButtonPalette {
    const fn of(fill: ButtonFill, tone: Tone) -> Self {
        match fill {
            ButtonFill::Solid => Self {
                border: ColorValue::Color(Color::TRANSPARENT),
                rest_bg: Some(tone.step(9)),
                rest_fg: ColorValue::Token(Token::OnSolid),
                hover_bg: tone.step(10),
                pressed_bg: tone.step(11),
                disabled_bg: Some(tone.step(8)),
                shadow: Some(Elevation::Sm),
            },
            ButtonFill::Soft => Self {
                border: ColorValue::Token(tone.step(6)),
                rest_bg: Some(tone.step(4)),
                rest_fg: ColorValue::Token(tone.step(11)),
                hover_bg: tone.step(5),
                pressed_bg: tone.step(6),
                disabled_bg: Some(tone.step(2)),
                shadow: None,
            },
            ButtonFill::Outline => Self {
                border: ColorValue::Token(tone.step(6)),
                rest_bg: None,
                rest_fg: ColorValue::Token(tone.step(11)),
                hover_bg: tone.step(4),
                pressed_bg: tone.step(5),
                disabled_bg: None,
                shadow: None,
            },
            ButtonFill::Ghost => Self {
                border: ColorValue::Color(Color::TRANSPARENT),
                rest_bg: None,
                rest_fg: ColorValue::Token(tone.step(11)),
                hover_bg: tone.step(4),
                pressed_bg: tone.step(5),
                disabled_bg: None,
                shadow: None,
            },
        }
    }
}

pub(super) fn style_button(node: NodeBuilder, fill: ButtonFill, tone: Tone) -> NodeBuilder {
    let palette = ButtonPalette::of(fill, tone);
    let mut node = node
        .tone_scope(tone)
        .radius(Radius::Lg)
        .transition(Transition::smooth())
        .border_width(BORDER_WIDTH)
        .border_color(palette.border)
        .text_color(palette.rest_fg);
    if let Some(bg) = palette.rest_bg {
        node = node.background(bg);
    }
    if let Some(elevation) = palette.shadow {
        node = node.shadow(elevation);
    }
    node.focus(move |s| s.background(palette.hover_bg))
        .hover(move |s| s.background(palette.hover_bg))
        .pressed(move |s| s.background(palette.pressed_bg))
        .disabled_paint(move |s| {
            let s = s.opacity(0.5);
            match palette.disabled_bg {
                Some(bg) => s.background(bg),
                None => s,
            }
        })
}

pub(super) fn apply_label_paint<P: Paint>(target: P, fill: ButtonFill, tone: Tone) -> P {
    let grid = Metrics::current().is_grid();
    let (color, weight) = match fill {
        ButtonFill::Solid => (
            ColorValue::Token(Token::OnSolid),
            if grid { 700 } else { 500 },
        ),
        ButtonFill::Soft | ButtonFill::Ghost | ButtonFill::Outline => {
            (ColorValue::Token(tone.step(11)), 500)
        }
    };
    let target = target.text_color(color).font_weight(weight);
    if fill == ButtonFill::Solid && grid {
        target
    } else {
        target.letter_spacing(-0.01)
    }
}

pub(super) fn label_node(
    label: TextContent,
    tone: Tone,
    fill: ButtonFill,
    compact: bool,
) -> TextBuilder {
    let node = apply_label_paint(text(label).nowrap().tone_scope(tone), fill, tone);
    if compact { node.class("text-xs") } else { node }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ButtonFill {
    Solid,
    Soft,
    Ghost,
    Outline,
}
