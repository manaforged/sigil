use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::{StyleAtom, Token};

const HAIRLINE_PX: f32 = 1.0;

static RULE_FILL: &[StyleAtom] = &[Token::Border.bg()];

#[must_use]
pub fn horizontal() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fill_width().height(HAIRLINE_PX))
        .class(RULE_FILL)
}

#[must_use]
pub struct VerticalDivider {
    height: Option<f32>,
}

pub const fn vertical() -> VerticalDivider {
    VerticalDivider { height: None }
}

impl VerticalDivider {
    pub const fn height(mut self, px: f32) -> Self {
        self.height = Some(px);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let layout = LayoutStyle::block().width(HAIRLINE_PX).shrink(0.0);
        let layout = match self.height {
            Some(px) => layout.height(px),
            None => layout.fill_height(),
        };
        container().hittable(false).layout(layout).class(RULE_FILL)
    }
}

impl From<VerticalDivider> for NodeBuilder {
    fn from(divider: VerticalDivider) -> Self {
        divider.into_node_builder()
    }
}

impl IntoNode for VerticalDivider {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
