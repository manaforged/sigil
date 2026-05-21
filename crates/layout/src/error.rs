//! Layout adapter errors.

use std::fmt::{Display, Formatter};

use kernel::NodeId;

/// Error returned by layout sync and compute operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// The owned layout engine rejected an operation.
    LayoutInvalid(String),
    /// A `sigil-kernel` node was missing from either `Cx` or the layout mirror.
    NodeMissing(NodeId),
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayoutInvalid(message) => write!(f, "layout error: {message}"),
            Self::NodeMissing(id) => write!(f, "layout node missing: {id:?}"),
        }
    }
}

impl std::error::Error for LayoutError {}
