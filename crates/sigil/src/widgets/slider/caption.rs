use crate::layout::{AlignItems, LayoutStyle, Orientation, Space};
use crate::node::{NodeBuilder, TextContent, container, text};
use crate::style::{ControlSize, Metrics};
use crate::widgets::Widget;

pub(super) fn with_caption(
    slider: NodeBuilder,
    caption: TextContent,
    orientation: Orientation,
    length: Option<f32>,
) -> NodeBuilder {
    let metrics = Metrics::current();
    let base = match orientation {
        Orientation::Horizontal => LayoutStyle::flex_row(),
        Orientation::Vertical => LayoutStyle::flex_column(),
    };
    let base = match (orientation, length) {
        (Orientation::Horizontal, None) => base.fill_width(),
        _ => base,
    };
    container()
        .layout(
            base.gap(Space::raw(
                metrics.control_gap(ControlSize::Default),
                "control gap",
            ))
            .align_items(AlignItems::Center),
        )
        .child(text(caption).nowrap())
        .child(slider)
}
