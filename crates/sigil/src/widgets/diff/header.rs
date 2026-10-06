use crate::kernel::{NodeBuilder, TextBuilder, container, text};
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle};
use crate::style::{BORDER_WIDTH, Paint, Token, space};

use super::super::button::{ButtonBuilder, button};
use super::rows::ROW_INSET;
use super::{DiffShape, DiffView};

const CHOICE_FONT_PX: f32 = 12.0;

fn paint_choice(choice: ButtonBuilder, active: bool) -> ButtonBuilder {
    let choice = choice.font_size(CHOICE_FONT_PX);
    if active {
        choice.text_color(Token::Gray12).bg(Token::Gray4)
    } else {
        choice.text_color(Token::Gray10)
    }
}

fn count(body: String, tone: Token) -> TextBuilder {
    text(body)
        .nowrap()
        .class("text-xs font-mono")
        .class(tone.text())
}

pub(super) fn header(shape: &DiffShape, added: usize, removed: usize) -> NodeBuilder {
    let mut row = container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(ROW_INSET)
                .min_size(0.0, 0.0)
                .align_items(AlignItems::Center)
                .gap(space(2)),
        )
        .child(
            text(shape.path.clone().unwrap_or_default())
                .nowrap()
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
                .class("text-xs font-mono font-bold")
                .class(Token::Gray12.text()),
        )
        .child(count(format!("+{added}"), Token::Success11))
        .child(count(format!("−{removed}"), Token::Error11));
    for choice in [DiffView::Split, DiffView::Unified] {
        let choice_button = paint_choice(button(choice.name()).ghost(), choice == shape.view);
        row = row.child(match shape.on_change.clone() {
            Some(on_change) => choice_button.activate(move || on_change(choice)),
            None => choice_button,
        });
    }
    row
}

pub(super) fn labels((lw, rw): (f32, f32)) -> NodeBuilder {
    let side = |label: &'static str, grow: f32, ruled: bool| {
        let half = container()
            .layout(
                LayoutStyle::flex_row()
                    .grow(grow)
                    .flex_basis(Dimension::Px(0.0))
                    .padding_x(ROW_INSET)
                    .min_size(0.0, 0.0),
            )
            .child(
                text(label)
                    .nowrap()
                    .class("text-xs")
                    .class(Token::Gray10.text()),
            );
        if ruled {
            half.border_right(BORDER_WIDTH.0)
                .border_color(Token::Border)
        } else {
            half
        }
    };
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .min_size(0.0, 0.0)
                .gap(0.0),
        )
        .child(side("Before", lw, true))
        .child(side("After", rw, false))
}
