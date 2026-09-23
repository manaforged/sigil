use crate::kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, text};
use crate::layout::prelude::{
    AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle,
};
use crate::style::NodeStyleExt;

const MARKER_WIDTH: f32 = 16.0;
const SELECTED_CLASS: &str = "bg-accent-5";

pub(super) fn row_height() -> f32 {
    crate::style::cell_size().unwrap_or(24.0)
}

#[derive(Default)]
pub struct ListRowSlots {
    pub marker: Option<TextContent>,
    pub status: Option<TextContent>,
    pub columns: Vec<(TextContent, f32)>,
    pub shrink: bool,
}

#[must_use]
pub fn list_row(
    selected: bool,
    title: impl Into<TextContent>,
    detail: impl Into<TextContent>,
) -> NodeBuilder {
    let detail: TextContent = detail.into();
    let empty = matches!(&detail, TextContent::Static(value) if value.is_empty());
    let slots = ListRowSlots {
        status: (!empty).then_some(detail),
        ..ListRowSlots::default()
    };
    list_row_with(selected, title, slots)
}

#[must_use]
pub fn list_row_with(
    selected: bool,
    title: impl Into<TextContent>,
    slots: ListRowSlots,
) -> NodeBuilder {
    let space = crate::style::cell_size().unwrap_or(8.0);
    let mut row = container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .padding_x(space)
                .gap(space)
                .align_items(AlignItems::Center),
        )
        .class("rounded-md");
    if let Some(marker) = slots.marker {
        row = row.child(
            container()
                .layout(
                    LayoutStyle::block()
                        .width(crate::style::cell_size().map_or(MARKER_WIDTH, |cell| cell * 2.0))
                        .shrink(0.0),
                )
                .child(text(marker).class("text-sm text-accent-11")),
        );
    }
    row = row.child(title_text(title.into(), slots.shrink));
    if let Some(status) = slots.status {
        row = row.child(status_text(status, slots.shrink));
    }
    for (value, width) in slots.columns {
        row = row.child(column(value, width, slots.shrink));
    }
    if selected {
        row = row.class(SELECTED_CLASS);
    }
    row
}

fn title_text(title: TextContent, shrink: bool) -> TextBuilder {
    let layout = LayoutStyle::block().grow(1.0).min_size(0.0, 0.0);
    text(title)
        .ellipsize()
        .layout(if shrink {
            layout
                .shrink(0.0)
                .max_size(Dimension::Percent(1.0), Dimension::Auto)
        } else {
            layout
        })
        .class("text-sm text-on-surface")
}

fn status_text(status: TextContent, shrink: bool) -> TextBuilder {
    let node = text(status).ellipsize().class("text-xs text-gray-11");
    if shrink {
        node.layout(LayoutStyle::block().shrink(1.0).min_size(0.0, 0.0))
    } else {
        node
    }
}

fn column(value: TextContent, width: f32, shrink: bool) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .width(width)
                .shrink(if shrink { 1.0 } else { 0.0 })
                .min_size(0.0, 0.0)
                .justify_content(JustifyContent::End),
        )
        .child(text(value).ellipsize().class("text-xs text-gray-11"))
}
