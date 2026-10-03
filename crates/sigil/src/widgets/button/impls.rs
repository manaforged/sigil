use crate::kernel::{Cx, IntoNode};
use crate::style::{PaintTarget, StateHost, StyleSlot};
use crate::widgets::layout_ext::{LayoutOverrides, WidgetLayoutExt};

use super::ButtonBuilder;

impl PaintTarget for ButtonBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for ButtonBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl StateHost for ButtonBuilder {}

impl IntoNode for ButtonBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}
