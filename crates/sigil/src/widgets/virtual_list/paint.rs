use crate::kernel::NodeBuilder;
use crate::style::{PaintTarget, Style, StyleSlot};

use super::{RowRender, VirtualListBuilder};

impl<T: 'static, R: RowRender<T>> PaintTarget for VirtualListBuilder<T, R> {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl<T: 'static, R: RowRender<T>> VirtualListBuilder<T, R> {
    pub(super) fn take_paint(&mut self) -> Style {
        std::mem::replace(&mut self.style_accumulator, Style::new())
    }
}

pub(super) fn apply(node: NodeBuilder, paint: Style) -> NodeBuilder {
    if paint.is_empty() {
        node
    } else {
        node.style(paint)
    }
}
