use crate::kernel::NodeBuilder;
use crate::layout::prelude::{Dimension, LayoutBuilderExt, LayoutStyle, Overflow};

use super::super::layout_ext::LayoutOverrides;

#[derive(Clone, Copy)]
pub(super) struct ViewportSize {
    pub(super) height: f32,
    pub(super) width: Option<f32>,
    pub(super) fill: bool,
}

pub(super) fn apply_viewport_overrides(
    root: NodeBuilder,
    overrides: &LayoutOverrides,
    size: ViewportSize,
) -> NodeBuilder {
    let base = LayoutStyle::block().overflow_y(Overflow::Hidden);
    let base = if size.fill {
        base.width(Dimension::Fill).height(Dimension::Fill)
    } else {
        let with_height = base.height(size.height);
        match size.width {
            Some(w) => with_height.width(w),
            None => with_height,
        }
    };
    root.layout(overrides.clone().apply(base))
}
