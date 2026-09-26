use crate::kernel::{IntoNode, NodeBuilder, TextContent, container, text};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::layout::snap;
use crate::reactive::Signal;
use crate::style::{NodeStyleExt, Size, StyleAtom, Token, space};

use super::super::button::icon_button;
use super::super::scroll::scroll_auto;

const TITLE_HEIGHT: f32 = 24.0;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CloseControl {
    Shown,
    Hidden,
}

static BODY: &[StyleAtom] = &[Token::Overlay.bg()];
const CLOSE_GLYPH: &str = "\u{00D7}";

pub(super) fn dialog_header(title: TextContent, close: Option<Signal<bool>>) -> NodeBuilder {
    let row = container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(snap::line(TITLE_HEIGHT))
                .shrink(0.0)
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(
            text(title)
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
                .class("text-lg font-medium")
                .class(Token::OnSurface.text()),
        );
    match close {
        Some(show) => row.child(
            icon_button(CLOSE_GLYPH, move || show.set(false))
                .accessible("Close")
                .size(Size::Compact),
        ),
        None => row,
    }
}

pub(super) fn dialog_body(children: Vec<Box<dyn IntoNode>>) -> NodeBuilder {
    let mut content = container().layout(LayoutStyle::flex_column().fill_width().gap(space(2)));
    for child in children {
        content = content.child(child);
    }
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .shrink(1.0)
                .min_size(0.0, 0.0),
        )
        .child(scroll_auto(content).fit().class(BODY))
}
