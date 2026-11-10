/// Orientation of a slider, splitter, or divider.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    /// The horizontal orientation.
    #[default]
    Horizontal,
    /// The vertical orientation.
    Vertical,
}
