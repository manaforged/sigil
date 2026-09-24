use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::prelude::{IntoPx, LayoutBuilderExt, LayoutStyle};
use crate::style::{BORDER_WIDTH, NodeStyleExt, StyleAtom, Token, space};

const SIDEBAR_DEFAULT: f32 = space(55);

static SIDEBAR_CHROME: &[StyleAtom] = &[
    Token::SurfaceSubtle.bg(),
    StyleAtom::BorderRight(BORDER_WIDTH),
    Token::Border.border(),
];

#[must_use]
pub fn sidebar(child: impl IntoNode + 'static) -> NodeBuilder {
    sidebar_width(SIDEBAR_DEFAULT, child)
}

#[must_use]
pub fn sidebar_width(width: impl IntoPx, child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::Navigation)
        .layout(
            LayoutStyle::flex_column()
                .width(width.into_px())
                .fill_height()
                .shrink(0.0)
                .padding(space(2))
                .gap(space(1)),
        )
        .class(SIDEBAR_CHROME)
        .child(child)
}
