/// The mouse cursor that the window shows while the pointer is over a node, named as the CSS `cursor` keywords are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CursorIcon {
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
    /// An arrow toward the right edge.
    EResize,
    /// An arrow toward the top edge.
    NResize,
    /// An arrow toward the top right corner.
    NeResize,
    /// An arrow toward the top left corner.
    NwResize,
    /// An arrow toward the bottom edge.
    SResize,
    /// An arrow toward the bottom right corner.
    SeResize,
    /// An arrow toward the bottom left corner.
    SwResize,
    /// An arrow toward the left edge.
    WResize,
    /// A two-way horizontal arrow.
    EwResize,
    /// A two-way vertical arrow.
    NsResize,
    /// A two-way arrow between the top right and bottom left corners.
    NeswResize,
    /// A two-way arrow between the top left and bottom right corners.
    NwseResize,
    /// A two-way horizontal arrow with a bar, for resizing a column or a vertical splitter.
    ColResize,
    /// A two-way vertical arrow with a bar, for resizing a row or a horizontal splitter.
    RowResize,
}

/// Edge or corner for `WindowCommand::DragResize`.
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
