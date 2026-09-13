use crate::kernel::{IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::NodeStyleExt;

const STATUS_BAR_PADDING: f32 = 8.0;
const STATUS_BAR_GAP: f32 = 12.0;
const KV_ROW_GAP: f32 = 8.0;

#[must_use]
pub fn status_bar<I>(children: I) -> NodeBuilder
where
    I: IntoIterator,
    I::Item: IntoNode + 'static,
{
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding(STATUS_BAR_PADDING)
                .gap(STATUS_BAR_GAP)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .class("bg-surface-hover");
    for child in children {
        root = root.child(child);
    }
    root
}

#[must_use]
pub fn kv_row(label: impl Into<TextContent>, value: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(KV_ROW_GAP)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(text(label).class("text-on-surface"))
        .child(value)
}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
