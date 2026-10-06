use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::style::{ControlSize, Metrics};
use crate::widgets::orientation::Orientation;

pub(super) fn with_caption(
    slider: NodeBuilder,
    caption: String,
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
            base.gap(metrics.gap(ControlSize::Default))
                .align_items(AlignItems::Center),
        )
        .child(text(caption).nowrap())
        .child(slider)
}
