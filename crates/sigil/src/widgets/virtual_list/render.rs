use crate::events::prelude::NodeEventsExt;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::prelude::{Dimension, LayoutBuilderExt, LayoutStyle, Overflow};

use super::selection::SelectionRuntime;

#[derive(Clone, Copy)]
pub(super) enum WidthSpec {
    Fixed(f32),
    Stretch,
}

pub(super) fn viewport_container(height: f32, width: WidthSpec) -> NodeBuilder {
    container().role(Role::ScrollView).layout(
        width.apply(
            LayoutStyle::block()
                .height(height)
                .overflow_y(Overflow::Hidden),
        ),
    )
}

pub(super) fn content_container(height: f32, width: WidthSpec) -> NodeBuilder {
    container().layout(width.apply(LayoutStyle::block().height(height)))
}

pub(super) fn place_row(
    index: usize,
    row_height: f32,
    width: WidthSpec,
    row: NodeBuilder,
    selection: SelectionRuntime,
) -> NodeBuilder {
    let slot = container()
        .layout(width.apply(LayoutStyle::block().height(row_height)))
        .child(row);
    with_selection(slot, index, selection)
}

pub(super) fn with_selection(
    mut slot: NodeBuilder,
    index: usize,
    selection: SelectionRuntime,
) -> NodeBuilder {
    if selection.idle() {
        return slot;
    }
    if let Some(selected) = selection.selected_signal(index) {
        slot = slot.selected(selected);
    }
    slot.pointer_down(move |event| {
        selection.apply_pointer_down(index, event);
    })
}

impl WidthSpec {
    fn apply(self, style: LayoutStyle) -> LayoutStyle {
        match self {
            Self::Fixed(px) => style.width(px),
            Self::Stretch => style.width(Dimension::Percent(1.0)),
        }
    }
}

pub(super) fn width_apply(width: WidthSpec, style: LayoutStyle) -> LayoutStyle {
    width.apply(style)
}

pub(super) fn width_spec(width: Option<f32>) -> WidthSpec {
    match width {
        Some(px) if px.is_finite() && px > 0.0 => WidthSpec::Fixed(px),
        _ => WidthSpec::Stretch,
    }
}
