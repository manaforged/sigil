use crate::kernel::{Node, NodeBuilder, container};
use crate::widgets::Widget;

use crate::layout::{LayoutStyle, Space, TrackSize};
use crate::style::{Metrics, StyleBinding};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Asymmetric(pub &'static str);

/// A container that places its children left to right, centred across the row, with the theme gap: 8 px in a window, one cell in a terminal.
#[must_use]
pub fn row() -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .stretch_width()
            .gap(Space::px(default_gap()))
            .align_items(crate::layout::AlignItems::Center),
    )
}

/// A container that places its children top to bottom with the theme gap.
#[must_use]
pub fn column() -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_column()
            .stretch_width()
            .gap(Space::px(default_gap())),
    )
}

/// A container that places its children in a grid with these column tracks.
#[must_use]
pub fn grid(columns: impl Into<Vec<TrackSize>>) -> NodeBuilder {
    container().layout(LayoutStyle::grid(columns.into()))
}

macro_rules! layout_setters {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*);)*) => {
        $($(#[$doc])* #[must_use] fn $name(self $(, $arg: $ty)*) -> Self {
            self.layout($crate::layout::LayoutStyle::default().$name($($arg),*))
        })*
    };
}

macro_rules! layout_methods {
    () => {
        $crate::layout::builder_ext::layout_setters! {
            /// Sets the space between the node's children.
            gap(gap: impl Into<$crate::layout::Space>);
            /// Sets the space inside the node's edges.
            padding(padding: impl Into<$crate::layout::Space>);
            /// Sets the node's smallest width.
            min_width(width: impl Into<$crate::layout::Length>);
            /// Sets the node's smallest height.
            min_height(height: impl Into<$crate::layout::Length>);
            /// Sets the node's largest width.
            max_width(width: impl Into<$crate::layout::Length>);
            /// Sets the node's largest height.
            max_height(height: impl Into<$crate::layout::Length>);
            /// Makes the node as wide as its parent.
            fill_width();
            /// Makes the node as high as its parent.
            fill_height();
            /// Makes the node as wide and as high as its parent.
            fill();
            /// Sets the node's flex shrink factor.
            shrink(factor: f32);
            /// Sets the node's size along the main axis before it grows or shrinks.
            flex_basis(basis: impl Into<$crate::layout::Length>);
            /// Sets the space inside the node's left and right edges.
            padding_x(padding: impl Into<$crate::layout::Space>);
            /// Sets the space inside the node's top and bottom edges.
            padding_y(padding: impl Into<$crate::layout::Space>);
            /// Sets the space inside the node's top edge.
            padding_top(padding: impl Into<$crate::layout::Space>);
            /// Sets the space inside the node's right edge.
            padding_right(padding: impl Into<$crate::layout::Space>);
            /// Sets the space inside the node's bottom edge.
            padding_bottom(padding: impl Into<$crate::layout::Space>);
            /// Sets the space inside the node's left edge.
            padding_left(padding: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's edges.
            margin(margin: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's left and right edges.
            margin_x(margin: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's top and bottom edges.
            margin_y(margin: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's top edge.
            margin_top(margin: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's right edge.
            margin_right(margin: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's bottom edge.
            margin_bottom(margin: impl Into<$crate::layout::Space>);
            /// Sets the space outside the node's left edge.
            margin_left(margin: impl Into<$crate::layout::Space>);
            /// Sets the horizontal space between the node's children.
            gap_x(gap: impl Into<$crate::layout::Space>);
            /// Sets the vertical space between the node's children.
            gap_y(gap: impl Into<$crate::layout::Space>);
            /// Sets what happens to content that is larger than the node.
            overflow(overflow: $crate::layout::Overflow);
            /// Sets what happens to content that is wider than the node.
            overflow_x(overflow: $crate::layout::Overflow);
            /// Sets what happens to content that is higher than the node.
            overflow_y(overflow: $crate::layout::Overflow);
            /// Sets the distance from each edge of the parent for an absolute node.
            inset(inset: impl Into<$crate::layout::Length>);
            /// Sets the distance from the parent's top edge for an absolute node.
            inset_top(top: impl Into<$crate::layout::Length>);
            /// Sets the distance from the parent's right edge for an absolute node.
            inset_right(right: impl Into<$crate::layout::Length>);
            /// Sets the distance from the parent's bottom edge for an absolute node.
            inset_bottom(bottom: impl Into<$crate::layout::Length>);
            /// Sets the distance from the parent's left edge for an absolute node.
            inset_left(left: impl Into<$crate::layout::Length>);
            /// Sets whether the node is placed in the flow or at its insets.
            position(position: $crate::layout::Position);
            /// Sets how the node lays out its children.
            display(display: $crate::layout::Display);
            /// Sets whether the node's children wrap onto more lines.
            flex_wrap(flex_wrap: $crate::layout::FlexWrap);
            /// Sets how the node's children are placed across the main axis.
            align_items(align_items: $crate::layout::AlignItems);
            /// Sets how the node's children are placed along the main axis.
            justify_content(justify_content: $crate::layout::JustifyContent);
            /// Sets how the node is placed across its parent's main axis.
            align_self(align_self: $crate::layout::AlignItems);
            /// Sets the node's grid column tracks.
            grid_columns(columns: Vec<$crate::layout::TrackSize>);
            /// Sets the node's grid row tracks.
            grid_rows(rows: Vec<$crate::layout::TrackSize>);
            /// Sets how many grid columns the node spans.
            grid_column_span(span: u16);
            /// Sets how many grid rows the node spans.
            grid_row_span(span: u16);
            /// Sets the grid column and row where the node starts.
            grid_area(column: u16, row: u16);
            /// Packs the node's grid children into earlier empty cells.
            grid_dense();
        }
    };
}

pub(crate) use {layout_methods, layout_setters};

fn default_gap() -> f32 {
    Metrics::current().control_gap(crate::style::ControlSize::Default)
}

pub(crate) fn asymmetric(node: NodeBuilder, reason: &'static str) -> NodeBuilder {
    node.mutate(move |node| {
        let _previous = node.attach(Asymmetric(reason));
    })
}

pub(crate) fn install_layout(node: &mut Node, style: &LayoutStyle) {
    let slots = style.slots();
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.strip(slots);
        binding.note_user(slots);
    }
    if let Some(existing) = node.get_extension_mut::<LayoutStyle>() {
        existing.merge(style);
    } else {
        let _previous = node.attach(style.clone());
    }
}
