use crate::kernel::{NodeBuilder, TextBuilder, container, text};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::style::{Token, space};

use super::DiffShape;
use super::rows::ROW_INSET;

fn count(body: String, tone: Token) -> TextBuilder {
    text(body)
        .nowrap()
        .class("text-xs font-mono")
        .class(tone.text())
}

pub(super) fn header(shape: &DiffShape, added: usize, removed: usize) -> NodeBuilder {
    container()
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
        .child(count(format!("−{removed}"), Token::Error11))
}
