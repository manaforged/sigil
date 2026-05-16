//! Layout adapter errors.

use std::fmt::{Display, Formatter};

use sigil_kernel::NodeId;

/// Error returned by layout sync and compute operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// Taffy rejected an adapter operation.
    TaffyInvalid(String),
    /// A `sigil-kernel` node was missing from either `Cx` or the Taffy mirror.
    NodeMissing(NodeId),
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TaffyInvalid(message) => write!(f, "taffy layout error: {message}"),
            Self::NodeMissing(id) => write!(f, "layout node missing: {id:?}"),
        }
    }
}

impl std::error::Error for LayoutError {}

impl From<taffy::TaffyError> for LayoutError {
    fn from(error: taffy::TaffyError) -> Self {
        Self::TaffyInvalid(format!("{error:?}"))
    }
}
