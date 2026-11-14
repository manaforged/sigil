use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::style::{StyleAtom, Token};
use crate::widgets::{Orientation, Widget};

const HAIRLINE_PX: f32 = 1.0;

static RULE_FILL: &[StyleAtom] = &[Token::Border.background()];

/// A 1 px rule; a horizontal one fills the width and a vertical one fills the height.
pub fn divider(orientation: Orientation) -> NodeBuilder {
    let thickness = crate::layout::snap::rule(HAIRLINE_PX);
    let layout = match orientation {
        Orientation::Horizontal => LayoutStyle::block()
            .fill_width()
            .height(thickness)
            .shrink(0.0),
        Orientation::Vertical => LayoutStyle::block()
            .width(thickness)
            .shrink(0.0)
            .fill_height(),
    };
    container().hittable(false).layout(layout).class(RULE_FILL)
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
