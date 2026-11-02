/// Orientation of a slider or splitter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    /// The horizontal orientation.
    #[default]
    Horizontal,
    /// The vertical orientation.
    Vertical,
}
