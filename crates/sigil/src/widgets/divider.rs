use crate::kernel::{NodeBuilder, container};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, StyleAtom};

const HAIRLINE_PX: f32 = 1.0;

static RULE_FILL: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];

#[must_use]
pub fn horizontal() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fill_width().height(HAIRLINE_PX))
        .class(RULE_FILL)
}

#[must_use]
pub fn vertical() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fill_height().width(HAIRLINE_PX))
        .class(RULE_FILL)
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
