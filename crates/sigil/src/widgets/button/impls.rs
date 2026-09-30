use crate::kernel::{Cx, IntoNode};
use crate::style::{StateHost, Style, StyleAccess};
use crate::widgets::layout_ext::{LayoutOverrides, WidgetLayoutExt};

use super::ButtonBuilder;

impl StyleAccess for ButtonBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
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
        self.into_node_builder().build(cx)
    }
}
