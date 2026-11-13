/// Orientation of a slider, splitter, or divider.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    /// The control runs from left to right; the default.
    #[default]
    Horizontal,
    /// The control runs from top to bottom.
    Vertical,
}
