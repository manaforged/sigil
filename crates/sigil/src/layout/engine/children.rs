use crate::kernel::{Cx, NodeId, NodeType};
use crate::style::StyleBinding;

use crate::layout::LayoutError;
use crate::layout::engine_drive::NodeHandle;
use crate::layout::text_style::{InheritedText, inherited_text_style};

use super::LayoutEngine;

impl LayoutEngine {
    pub(super) fn sync_child_list(
        &mut self,
        cx: &Cx,
        id: NodeId,
        node: &crate::kernel::Node,
    ) -> Result<(), LayoutError> {
        let parent = if matches!(node.kind, NodeType::Fragment) {
            Self::nearest_mirrored_ancestor(cx, id)
        } else {
            Some(id)
        };
        let Some(parent) = parent else {
            return Ok(());
        };
        let Some(parent_node) = cx.get(parent) else {
            return Ok(());
        };
        if parent_node
            .get_extension::<StyleBinding>()
            .is_some_and(StyleBinding::hides)
        {
            self.drive.set_children(self.handle(parent)?, &[])?;
            self.hidden.insert(parent);
            return Ok(());
        }
        if self.hidden.remove(&parent) {
            let text = inherited_text_style(cx, parent);
            return self
                .sync_subtree(cx, parent, InheritedText::new(&text))
                .map(|_handles| ());
        }
        let handles = self.flatten_child_handles(cx, parent_node.children())?;
        self.drive.set_children(self.handle(parent)?, &handles)
    }

    fn nearest_mirrored_ancestor(cx: &Cx, id: NodeId) -> Option<NodeId> {
        let mut current = cx.parent(id);
        while let Some(parent) = current {
            let node = cx.get(parent)?;
            if !matches!(node.kind, NodeType::Fragment) {
                return Some(parent);
            }
            current = cx.parent(parent);
        }
        None
    }

    fn flatten_child_handles(
        &mut self,
        cx: &Cx,
        children: &[NodeId],
    ) -> Result<Vec<NodeHandle>, LayoutError> {
        let mut handles = Vec::new();
        for child in children {
            let node = cx.get(*child).ok_or(LayoutError::NodeMissing(*child))?;
            if matches!(node.kind, NodeType::Fragment) {
                handles.extend(self.flatten_child_handles(cx, node.children())?);
            } else if let Some(handle) = self.core_to_handle.get(child.key()).copied() {
                handles.push(handle);
            } else {
                let text = inherited_text_style(cx, *child);
                let inherited = InheritedText::new(&text);
                handles.extend(self.sync_subtree(cx, *child, inherited)?);
            }
        }
        Ok(handles)
    }
}
