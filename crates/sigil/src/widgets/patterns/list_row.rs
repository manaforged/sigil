use crate::kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, text};
use crate::layout::prelude::{
    AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle, cols,
};
use crate::reactive::{Signal, signal};
use crate::style::{Fill, NodeStyleExt, Radius, StateVariants, Token, space};

use super::super::interactive::interactive;
use super::super::popup::{ChevronDirection, chevron};

const INDENT_COLS: f32 = 2.0;

pub(super) fn row_height() -> f32 {
    crate::layout::snap::line(24.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disclosure {
    #[default]
    None,
    Leaf,
    Collapsed,
    Expanded,
}

#[derive(Default)]
pub struct ListRowSlots {
    pub depth: u16,
    pub disclosure: Disclosure,
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
    let mut row = row_frame(slots.depth, slots.disclosure);
    if let Some(marker) = slots.marker {
        row = row.child(
            container()
                .layout(LayoutStyle::block().width(cols(2.0)).shrink(0.0))
                .child(text(marker).class("text-sm").class(Token::Accent11.text())),
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
        return row
            .selected(signal(true))
            .class([Radius::Md.atom(), ROW_SELECTED.bg()]);
    }
    row_states(row, None)
}

pub(crate) const ROW_SELECTED: Token = Token::Accent5;

pub(crate) fn row_states(row: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let row = interactive(row.class(Radius::Md.atom()))
        .hover(|s| s.bg(Token::SurfaceHover))
        .pressed(|s| s.bg(Token::SurfacePressed));
    match disabled {
        Some(disabled) => row.disabled_signal(disabled),
        None => row,
    }
    .into_node_builder()
}

fn row_frame(depth: u16, disclosure: Disclosure) -> NodeBuilder {
    let pad = space(2);
    let mut row = container().role(Role::GenericContainer).layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(row_height())
            .padding_x(pad)
            .gap(pad)
            .align_items(AlignItems::Center),
    );
    if depth > 0 {
        row = row.child(indent(depth));
    }
    if matches!(disclosure, Disclosure::Collapsed | Disclosure::Expanded) {
        row = row.expanded(signal(disclosure == Disclosure::Expanded));
    }
    if disclosure != Disclosure::None {
        row = row.child(disclosure_slot(disclosure));
    }
    row
}

fn indent(depth: u16) -> NodeBuilder {
    container().layout(
        LayoutStyle::block()
            .width(cols(INDENT_COLS * f32::from(depth)))
            .shrink(0.0),
    )
}

fn disclosure_slot(disclosure: Disclosure) -> NodeBuilder {
    let slot = container().layout(
        LayoutStyle::flex_row()
            .width(cols(INDENT_COLS))
            .shrink(0.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    );
    match disclosure {
        Disclosure::Collapsed => slot.child(chevron(ChevronDirection::Right, space(3))),
        Disclosure::Expanded => slot.child(chevron(ChevronDirection::Down, space(3))),
        Disclosure::None | Disclosure::Leaf => slot,
    }
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
        .class("text-sm")
        .class(Token::OnSurface.text())
}

fn status_text(status: TextContent, shrink: bool) -> TextBuilder {
    let node = text(status)
        .ellipsize()
        .class("text-xs")
        .class(Token::Gray11.text());
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
        .child(
            text(value)
                .ellipsize()
                .class("text-xs")
                .class(Token::Gray11.text()),
        )
}
