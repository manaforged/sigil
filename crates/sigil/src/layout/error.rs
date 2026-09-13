use std::fmt::{Display, Formatter};

use crate::kernel::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    LayoutInvalid(String),
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
