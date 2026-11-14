/// The mouse cursor that the window shows while the pointer is over a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Cursor {
    /// The platform arrow.
    #[default]
    Default,
    /// A pointing hand, for links and buttons.
    Pointer,
    /// A text beam, for text that can be edited or selected.
    Text,
    /// An open hand, for something that can be dragged.
    Grab,
    /// A closed hand, while something is dragged.
    Grabbing,
    /// A crosshair, for picking a precise point.
    Crosshair,
    /// A slashed circle, for an action that is not allowed.
    NotAllowed,
    /// Arrows in four directions, for something that moves.
    Move,
    /// A busy indicator.
    Wait,
    /// An arrow toward an edge or a corner, for resizing from it.
    Resize(ResizeDirection),
    /// A two-way horizontal arrow, for resizing a column or a vertical splitter.
    ColumnResize,
    /// A two-way vertical arrow, for resizing a row or a horizontal splitter.
    RowResize,
}

/// Edge or corner for `WindowCommand::DragResize` and `Cursor::Resize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResizeDirection {
    /// The right edge.
    East,
    /// The top edge.
    North,
    /// The top right corner.
    NorthEast,
    /// The top left corner.
    NorthWest,
    /// The bottom edge.
    South,
    /// The bottom right corner.
    SouthEast,
    /// The bottom left corner.
    SouthWest,
    /// The left edge.
    West,
}
