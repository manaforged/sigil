use events::prelude::NodeEventsExt;
use kernel::{NodeBuilder, Role, container};
use layout::prelude::{Dimension, LayoutBuilderExt, LayoutStyle, Overflow, Position};

use super::selection::SelectionRuntime;
use super::window::{VisibleWindow, prefix_row_top, row_top, total_height};

pub(super) type HeightSpec<'a> = (f32, Option<&'a [f32]>);

#[derive(Clone, Copy)]
pub(super) struct VisibleRenderSpec<'a> {
    heights: HeightSpec<'a>,
    width: WidthSpec,
    selection: SelectionRuntime,
}

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

/// Content wrapper for the reactive (`items_bind`) path, which renders
/// the full bound `Vec` and re-builds it on every signal change. Its
/// height is **auto** (sized to the rebuilt children) rather than a
/// build-time row-count snapshot — a fixed height taken when the bound
/// signal was empty collapses the list to its first row under the
/// incremental layout path. The enclosing viewport clips overflow.
pub(super) fn content_container_auto(width: WidthSpec) -> NodeBuilder {
    container().layout(width.apply(LayoutStyle::block()))
}

pub(super) fn build_visible_window<T, R>(
    items: &[T],
    render: &R,
    window: VisibleWindow,
    spec: VisibleRenderSpec<'_>,
) -> NodeBuilder
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let (row_height, row_tops) = spec.heights;
    let top = row_tops.map_or_else(
        || row_top(window.first, row_height),
        |row_tops| prefix_row_top(row_tops, window.first),
    );
    let visible_height = row_tops.map_or_else(
        || total_height(window.len(), row_height),
        |row_tops| prefix_row_top(row_tops, window.end) - top,
    );
    let mut visible = container().layout(
        spec.width.apply(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .inset_top(top)
                .inset_left(0.0)
                .height(visible_height),
        ),
    );

    for index in window.first..window.end {
        let Some(item) = items.get(index) else {
            break;
        };
        let slot_height = row_tops.map_or(row_height, |row_tops| {
            prefix_row_top(row_tops, index + 1) - prefix_row_top(row_tops, index)
        });
        visible = visible.child(row_slot(
            index,
            slot_height,
            spec.width,
            render(index, item),
            spec.selection,
        ));
    }

    visible
}

pub(super) fn row_slot(
    index: usize,
    row_height: f32,
    width: WidthSpec,
    row: NodeBuilder,
    selection: SelectionRuntime,
) -> NodeBuilder {
    let mut slot = container()
        .layout(width.apply(LayoutStyle::block().height(row_height)))
        .child(row);
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

/// Module-public adapter for [`WidthSpec::apply`] so callers in
/// `mod.rs` can decorate a [`LayoutStyle`] without exposing the
/// per-axis enum's private method.
pub(super) fn width_apply(width: WidthSpec, style: LayoutStyle) -> LayoutStyle {
    width.apply(style)
}

pub(super) fn width_spec(width: Option<f32>) -> WidthSpec {
    match width {
        Some(px) if px.is_finite() && px > 0.0 => WidthSpec::Fixed(px),
        _ => WidthSpec::Stretch,
    }
}

impl<'a> VisibleRenderSpec<'a> {
    pub(super) const fn new(
        heights: HeightSpec<'a>,
        width: WidthSpec,
        selection: SelectionRuntime,
    ) -> Self {
        Self {
            heights,
            width,
            selection,
        }
    }
}
