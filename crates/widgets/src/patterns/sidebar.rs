use sigil_kernel::{IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_style::NodeStyleExt;

const SIDEBAR_DEFAULT: f32 = 220.0;

#[must_use]
pub fn sidebar(child: impl IntoNode + 'static) -> NodeBuilder {
    sidebar_width(SIDEBAR_DEFAULT, child)
}

#[must_use]
pub fn sidebar_width(width: f32, child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::Navigation)
        .layout(LayoutStyle::flex_column().width(width).fill())
        .class("bg-surface-hover")
        .child(child)
}
