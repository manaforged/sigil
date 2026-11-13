use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::style::{StyleAtom, Token};
use crate::widgets::{Layout, Orientation};

const HAIRLINE_PX: f32 = 1.0;

static RULE_FILL: &[StyleAtom] = &[Token::Border.background()];

/// A 1 px rule; a horizontal one fills the width and a vertical one fills the height.
pub fn divider(orientation: Orientation) -> NodeBuilder {
    let layout = match orientation {
        Orientation::Horizontal => LayoutStyle::block().fill_width().height(HAIRLINE_PX),
        Orientation::Vertical => LayoutStyle::block()
            .width(HAIRLINE_PX)
            .shrink(0.0)
            .fill_height(),
    };
    container().hittable(false).layout(layout).class(RULE_FILL)
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
