use super::{LayoutForest, LocalId};
use crate::layout::{LayoutError, StyleData};

impl LayoutForest {
    pub(crate) fn free(&mut self, id: LocalId) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        self.detach(id);
        let Some(entry) = self.nodes.get_mut(slot) else {
            return Err(stale(id));
        };
        let next_tag = entry.tag.wrapping_add(1);
        let children = std::mem::take(&mut entry.children);
        entry.alive = false;
        entry.kernel = None;
        entry.style = StyleData::default();
        entry.text = None;
        entry.parent = None;
        entry.tag = next_tag;
        self.orphan(id, &children);
        self.forget(slot);
        self.free.push(LocalId {
            index: id.index,
            tag: next_tag,
        });
        Ok(())
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn attach(&mut self, parent: LocalId, child: LocalId) -> Result<(), LayoutError> {
        let _ = self.slot(child)?;
        let slot = self.slot(parent)?;
        let Some(entry) = self.nodes.get_mut(slot) else {
            return Err(stale(parent));
        };
        entry.children.push(child);
        self.adopt(parent, &[child]);
        self.invalidate(parent);
        Ok(())
    }

    pub(crate) fn set_children(
        &mut self,
        parent: LocalId,
        children: &[LocalId],
    ) -> Result<(), LayoutError> {
        for child in children {
            let _ = self.slot(*child)?;
        }
        let slot = self.slot(parent)?;
        let Some(entry) = self.nodes.get_mut(slot) else {
            return Err(stale(parent));
        };
        if entry.children.as_slice() == children {
            return Ok(());
        }
        entry.children.clear();
        entry.children.extend_from_slice(children);
        self.adopt(parent, children);
        self.invalidate(parent);
        Ok(())
    }

    fn detach(&mut self, id: LocalId) {
        let Some(parent) = self.node(id).and_then(|node| node.parent) else {
            return;
        };
        if self.node(parent).is_none() {
            return;
        }
        if let Some(entry) = self.nodes.get_mut(parent.index as usize) {
            entry.children.retain(|child| *child != id);
        }
        self.invalidate(parent);
    }

    fn orphan(&mut self, parent: LocalId, children: &[LocalId]) {
        for child in children {
            if let Some(node) = self.nodes.get_mut(child.index as usize)
                && node.alive
                && node.tag == child.tag
                && node.parent == Some(parent)
            {
                node.parent = None;
            }
        }
    }
}

fn stale(id: LocalId) -> LayoutError {
    LayoutError::LayoutInvalid(format!(
        "stale layout-forest id index={} tag={}",
        id.index, id.tag
    ))
}
