//! Convenient single-import surface for layout users.

pub use crate::{
    Dimension, LayoutBuilderExt, LayoutEngine, LayoutError, LayoutResult, LayoutStyle, Overflow,
    StyleAlignItems as AlignItems, StyleAlignSelf as AlignSelf, StyleDisplay as Display,
    StyleFlexDirection as FlexDirection, StyleJustifyContent as JustifyContent,
    StyleLengthPercentage as LengthPercentage, StyleLengthPercentageAuto as LengthPercentageAuto,
    StylePosition as Position, TextMeasure,
};
pub use hal::{Dim, LogicalSize, Size};

/// Compatibility helper for pre-cutover code that imported a length helper.
#[must_use]
pub const fn length(value: f32) -> LengthPercentage {
    LengthPercentage::Px(value)
}
