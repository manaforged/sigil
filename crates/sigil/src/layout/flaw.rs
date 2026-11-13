/// The rule that an `Issue` breaks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Flaw {
    /// Rows of a table whose columns do not line up.
    Column,
    /// A lone child with nearly, but not exactly, even space around it.
    Inset,
    /// Text taller than the box that clips it.
    ClippedText,
    /// Controls in one row with different heights.
    UnevenControls,
    /// Siblings in normal flow that cover each other.
    Overlap,
    /// A child that leaves a parent that does not clip or scroll.
    Overflow,
    /// Text below the WCAG 2.2 AA contrast ratio.
    Contrast,
    /// A control under the WCAG 2.2 AA pointer target size.
    SmallTarget,
}
