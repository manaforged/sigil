use core::fmt;

use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::layout::Orientation;

pub use crate::layout::Flaw;

/// A design rule that the laid-out tree breaks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Issue {
    /// Rows of a table whose columns do not line up.
    Column {
        /// The stack whose child rows hold the cells.
        parent: NodeLabel,
        /// The 0-based index of the column that is out of line.
        column: usize,
        /// The y position of each row, in pixels.
        tops: Vec<f32>,
        /// The x position where the cell of each row starts, in pixels.
        starts: Vec<f32>,
        /// The x position where the cell of each row ends, in pixels.
        ends: Vec<f32>,
    },
    /// A lone child with nearly, but not exactly, even space around it.
    Inset {
        /// The lone child with uneven space around it.
        node: NodeLabel,
        /// The child's box.
        rect: Rect,
        /// The axis along which the space is uneven.
        axis: Orientation,
        /// The space between the start edge of the parent and the child, in pixels.
        start: f32,
        /// The space between the child and the end edge of the parent, in pixels.
        end: f32,
    },
    /// Text taller than the box that clips it.
    ClippedText {
        /// The text node that is clipped.
        node: NodeLabel,
        /// The height that the text needs, in pixels.
        needed: f32,
        /// The height of the box that clips the text, in pixels.
        shown: f32,
    },
    /// Controls in one row with different heights.
    UnevenControls {
        /// The row that holds the controls.
        parent: NodeLabel,
        /// The height of each control in the row, in pixels.
        heights: Vec<f32>,
    },
    /// Siblings in normal flow that cover each other.
    Overlap {
        /// The earlier of the two siblings in child order.
        first: NodeLabel,
        /// The later of the two siblings in child order.
        second: NodeLabel,
    },
    /// A child that leaves a parent that does not clip or scroll.
    Overflow {
        /// The child that leaves its parent.
        node: NodeLabel,
        /// The parent that the child leaves.
        parent: NodeLabel,
        /// The largest distance that the child extends past an edge of the parent, in pixels.
        by: f32,
    },
    /// Text below the WCAG 2.2 AA contrast ratio.
    Contrast {
        /// The text node with too little contrast.
        node: NodeLabel,
        /// The contrast ratio between the text and the color behind it.
        ratio: f32,
        /// The lowest contrast ratio that passes for this text.
        needed: f32,
    },
    /// A control under the WCAG 2.2 AA pointer target size.
    SmallTarget {
        /// The control that is too small.
        node: NodeLabel,
        /// The width of the control, in pixels.
        width: f32,
        /// The height of the control, in pixels.
        height: f32,
        /// The smallest width and height that pass, in pixels.
        needed: f32,
    },
}

impl Issue {
    /// Returns the rule that this issue breaks.
    #[must_use]
    pub const fn kind(&self) -> Flaw {
        match self {
            Self::Column { .. } => Flaw::Column,
            Self::Inset { .. } => Flaw::Inset,
            Self::ClippedText { .. } => Flaw::ClippedText,
            Self::UnevenControls { .. } => Flaw::UnevenControls,
            Self::Overlap { .. } => Flaw::Overlap,
            Self::Overflow { .. } => Flaw::Overflow,
            Self::Contrast { .. } => Flaw::Contrast,
            Self::SmallTarget { .. } => Flaw::SmallTarget,
        }
    }

    /// The node to fix: the parent for column and control issues, the first node of an overlap.
    #[must_use]
    pub const fn node(&self) -> &NodeLabel {
        match self {
            Self::Column { parent, .. } | Self::UnevenControls { parent, .. } => parent,
            Self::Overlap { first, .. } => first,
            Self::Inset { node, .. }
            | Self::ClippedText { node, .. }
            | Self::Overflow { node, .. }
            | Self::Contrast { node, .. }
            | Self::SmallTarget { node, .. } => node,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeLabel {
    pub id: NodeId,
    pub label: String,
}

impl fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} #{}>", self.label, self.id.to_accesskit().0)
    }
}

impl Issue {
    fn fmt_column(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Column {
            parent,
            column,
            tops,
            starts,
            ends,
        } = self
        else {
            return Ok(());
        };
        write!(
            f,
            "column {column} of {parent} in rows at y {tops:?} starts at {starts:?} and ends at {ends:?}: give the cells of the column one width, or use a grid"
        )
    }

    fn fmt_inset(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Inset {
            node,
            rect,
            axis,
            start,
            end,
        } = self
        else {
            return Ok(());
        };
        write!(
            f,
            "{node} at ({}, {}) sized {}x{} is inset {start} at the start and {end} at the end on axis {axis:?}",
            rect.origin.x, rect.origin.y, rect.width, rect.height
        )
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Column { .. } => self.fmt_column(f),
            Self::Inset { .. } => self.fmt_inset(f),
            Self::ClippedText {
                node,
                needed,
                shown,
            } => write!(
                f,
                "{node} needs {needed} of height and its clip shows {shown}: let the box grow, or use a smaller TextRole"
            ),
            Self::UnevenControls { parent, heights } => write!(
                f,
                "the controls in {parent} have heights {heights:?}: give them one ControlSize"
            ),
            Self::Overlap { first, second } => {
                write!(
                    f,
                    "{first} and {second} cover each other: check their sizes and margins"
                )
            }
            Self::Overflow { node, parent, by } => write!(
                f,
                "{node} leaves {parent} by {by}: let the parent grow, wrap the content, or scroll it"
            ),
            Self::Contrast {
                node,
                ratio,
                needed,
            } => write!(
                f,
                "{node} has a contrast of {ratio:.2}:1 and needs {needed}:1: use a theme Token"
            ),
            Self::SmallTarget {
                node,
                width,
                height,
                needed,
            } => write!(
                f,
                "{node} is {width}x{height} and needs {needed} on each side: use a larger ControlSize"
            ),
        }
    }
}
