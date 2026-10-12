use crate::kernel::{IntoNode, NodeBuilder, TextContent, container, text};
use crate::layout::snap;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{ControlSize, Paint, Token, space};

use super::super::button::icon_button;
use super::super::scroll::{SCROLLBAR_WIDTH, scroll};

pub(super) const BODY_GUTTER: f32 = SCROLLBAR_WIDTH;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CloseControl {
    Shown,
    Hidden,
}

pub(super) fn dialog_header(title: TextContent, close: Option<Signal<bool>>) -> NodeBuilder {
    let row = container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(snap::line(24.0))
                .shrink(0.0)
                .padding_right(BODY_GUTTER)
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
            icon_button("\u{00D7}", move || show.set(false))
                .accessible(theme_label(StringKey::Close))
                .size(ControlSize::Compact),
        ),
        None => row,
    }
}

pub(super) fn dialog_body(children: Vec<Box<dyn IntoNode>>, offset: Signal<f32>) -> NodeBuilder {
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
        .child(scroll(content).fit().scroll_bind(offset).bg(Token::Overlay))
}
