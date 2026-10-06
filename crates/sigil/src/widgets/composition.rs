use crate::kernel::{IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{BORDER_WIDTH, ControlSize, Metrics, StyleAtom, Token, space};

pub(crate) const STATUS_BAR_HEIGHT: f32 = space(7);
const STATUS_BAR_GAP: f32 = space(3);
const KV_ROW_GAP: f32 = space(3);
const KV_KEY_COLS: f32 = 16.0;

pub(crate) static STATUS_CHROME: &[StyleAtom] = &[
    Token::SurfaceSubtle.bg(),
    StyleAtom::BorderTop(BORDER_WIDTH),
    Token::Border.border(),
];

#[must_use]
pub fn status_bar<I>(children: I) -> NodeBuilder
where
    I: IntoIterator,
    I::Item: IntoNode + 'static,
{
    let children: Vec<I::Item> = children.into_iter().collect();
    let last = children.len().checked_sub(1).filter(|last| *last > 0);
    let mut root = container()
        .role(Role::Status)
        .label(theme_label(StringKey::StatusBar))
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(crate::layout::snap::ruled(STATUS_BAR_HEIGHT))
                .shrink(0.0)
                .padding_x(space(2))
                .gap(STATUS_BAR_GAP)
                .align_items(AlignItems::Center),
        )
        .class(STATUS_CHROME);
    for (index, child) in children.into_iter().enumerate() {
        root = root.child(status_slot(Some(index) == last).child(child));
    }
    root
}

fn status_slot(trailing: bool) -> NodeBuilder {
    let layout = LayoutStyle::flex_row()
        .gap(STATUS_BAR_GAP)
        .align_items(AlignItems::Center);
    container().layout(if trailing {
        layout.grow(1.0).justify_content(JustifyContent::End)
    } else {
        layout.shrink(0.0)
    })
}

#[must_use]
pub fn kv_row(label: impl Into<TextContent>, value: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .min_size(0.0, Metrics::current().control_height(ControlSize::Default))
                .gap(KV_ROW_GAP)
                .align_items(AlignItems::Center),
        )
        .child(
            text(label)
                .nowrap()
                .ellipsize()
                .layout(LayoutStyle::block().width(cols(KV_KEY_COLS)).shrink(0.0))
                .class("text-sm")
                .class(Token::OnSurfaceMuted.text()),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .grow(1.0)
                        .min_size(0.0, 0.0)
                        .align_items(AlignItems::Center),
                )
                .class("text-sm")
                .class(Token::OnSurface.text())
                .child(value),
        )
}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
