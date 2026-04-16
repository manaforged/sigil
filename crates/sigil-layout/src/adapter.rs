//! `sigil-hal::Layout` implementation.

use sigil_core::NodeId;
use sigil_hal::{Layout, Size};

use crate::{LayoutEngine, LayoutError, LayoutIntent, LayoutResult};

impl Layout for LayoutEngine {
    type Error = LayoutError;
    type NodeId = NodeId;
    type Intent = LayoutIntent;
    type Result = LayoutResult;

    fn upsert(&mut self, id: Self::NodeId, intent: &Self::Intent) -> Result<(), Self::Error> {
        self.upsert_mirror(id, intent)
    }

    fn set_children(
        &mut self,
        parent: Self::NodeId,
        children: &[Self::NodeId],
    ) -> Result<(), Self::Error> {
        self.set_children_mirror(parent, children)
    }

    fn remove(&mut self, id: Self::NodeId) -> Result<(), Self::Error> {
        self.remove_mirror(id)
    }

    fn compute(&mut self, root: Self::NodeId, available: Size) -> Result<(), Self::Error> {
        self.compute_mirror(root, available)
    }

    fn result(&self, id: Self::NodeId) -> Option<Self::Result> {
        Self::result(self, id)
    }
}
