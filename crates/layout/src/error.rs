//! Layout adapter errors.

use std::fmt::{Display, Formatter};

use sigil_kernel::NodeId;

/// Error returned by layout sync and compute operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// The owned layout engine rejected an operation.
    LayoutInvalid(String),
    /// A `sigil-kernel` node was missing from either `Cx` or the layout mirror.
    NodeMissing(NodeId),
    /// A node requested grid layout, which is not supported in 0.1.x.
    /// Previously this silently fell back to block (removed per D-U4 —
    /// a silent fallback is worse than a loud rejection).
    GridUnsupported(NodeId),
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayoutInvalid(message) => write!(f, "layout error: {message}"),
            Self::NodeMissing(id) => write!(f, "layout node missing: {id:?}"),
            Self::GridUnsupported(id) => write!(
                f,
                "grid layout is not supported in 0.1.x (node {id:?}); \
                 use flex or block — a real grid solver is a later program"
            ),
        }
    }
}

impl std::error::Error for LayoutError {}
