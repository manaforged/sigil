use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::style::{StyleAtom, Token};
use crate::widgets::Widget;

const HAIRLINE_PX: f32 = 1.0;

static RULE_FILL: &[StyleAtom] = &[Token::Border.background()];

#[must_use]
/// A horizontal divider.
pub fn horizontal() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fill_width().height(HAIRLINE_PX))
        .class(RULE_FILL)
}

#[must_use]
/// A vertical divider that fills the height.
pub fn vertical() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(
            LayoutStyle::block()
                .width(HAIRLINE_PX)
                .shrink(0.0)
                .fill_height(),
        )
        .class(RULE_FILL)
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
