#![allow(clippy::redundant_pub_crate)]

use crate::kernel::{NodeId, TextContent};
use crate::style::TextStyle;
use smallvec::SmallVec;

#[cfg(test)]
use crate::layout::TextMeasure;
use crate::layout::{LayoutError, StyleData};

mod box_cell;
mod default_style;
mod intrinsic;
mod measure;
mod text_memo;

pub(crate) use box_cell::BoxCell;
use default_style::DEFAULT_STYLE;
pub(crate) use intrinsic::IntrinsicKey;
use intrinsic::IntrinsicMemo;
pub(crate) use measure::{AvailableSpace, MeasureScope, SpaceConstraint};
use text_memo::TextMemo;
#[cfg(test)]
use text_memo::{TEXT_MEMO_LIVE, TextMemoKey};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct LocalId {
    index: u32,
    tag: u32,
}

impl LocalId {
    pub(crate) const fn slot(self) -> u32 {
        self.index
    }

    pub(crate) const fn era(self) -> u32 {
        self.tag
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LayoutNode {
    kernel: Option<NodeId>,
    style: StyleData,
    text: Option<LayoutText>,
    children: SmallVec<[LocalId; 4]>,
    parent: Option<LocalId>,
    tag: u32,
    alive: bool,
}

impl LayoutNode {
    pub(crate) const fn kernel(&self) -> Option<NodeId> {
        self.kernel
    }

    pub(crate) const fn style(&self) -> &StyleData {
        &self.style
    }

    pub(crate) const fn text(&self) -> Option<&LayoutText> {
        self.text.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LayoutText {
    pub(crate) content: TextContent,
    pub(crate) style: Option<TextStyle>,
    pub(crate) wrap: bool,
    pub(crate) ellipsis: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TextFlags {
    pub(crate) wrap: bool,
    pub(crate) ellipsis: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ComputedBox {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

#[derive(Debug)]
pub(crate) struct LayoutForest {
    nodes: Vec<LayoutNode>,
    boxes: Vec<BoxCell>,
    free: Vec<LocalId>,
    frame: u32,
    text_memo: std::cell::RefCell<TextMemo>,
    intrinsic: Vec<IntrinsicMemo>,
}

impl Default for LayoutForest {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutForest {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            boxes: Vec::new(),
            free: Vec::new(),
            frame: 1,
            text_memo: std::cell::RefCell::new(TextMemo::default()),
            intrinsic: Vec::new(),
        }
    }

    pub(crate) const fn next_frame(&mut self) -> u32 {
        self.frame = self.frame.wrapping_add(1);
        self.frame
    }

    pub(crate) const fn current_frame(&self) -> u32 {
        self.frame
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    pub(crate) fn insert(&mut self, kernel: Option<NodeId>, style: StyleData) -> LocalId {
        if let Some(slot) = self.free.pop() {
            let idx = slot.index as usize;
            let node = LayoutNode {
                kernel,
                style: crate::layout::units::resolve_style(style),
                text: None,
                children: SmallVec::new(),
                parent: None,
                tag: slot.tag,
                alive: true,
            };
            let node_slot = self
                .nodes
                .get_mut(idx)
                .expect("free-list slot points inside nodes slab");
            *node_slot = node;
            let box_slot = self
                .boxes
                .get_mut(idx)
                .expect("free-list slot points inside boxes slab");
            box_slot.reset();
            self.forget(idx);
            return slot;
        }
        let index = u32::try_from(self.nodes.len()).expect("forest slab exceeds u32::MAX");
        let id = LocalId { index, tag: 0 };
        self.nodes.push(LayoutNode {
            kernel,
            style: crate::layout::units::resolve_style(style),
            text: None,
            children: SmallVec::new(),
            parent: None,
            tag: 0,
            alive: true,
        });
        self.boxes.push(BoxCell::new(ComputedBox::default()));
        self.intrinsic.push(IntrinsicMemo::default());
        id
    }

    pub(crate) fn free(&mut self, id: LocalId) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        let next_tag = entry.tag.wrapping_add(1);
        entry.alive = false;
        entry.children.clear();
        entry.kernel = None;
        entry.style = StyleData::default();
        entry.text = None;
        entry.parent = None;
        entry.tag = next_tag;
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
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the parent index is in-bounds");
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
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the parent index is in-bounds");
        if entry.children.as_slice() == children {
            return Ok(());
        }
        entry.children.clear();
        entry.children.extend_from_slice(children);
        self.adopt(parent, children);
        self.invalidate(parent);
        Ok(())
    }

    pub(crate) fn set_style(&mut self, id: LocalId, style: StyleData) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        let resolved = crate::layout::units::resolve_style(style);
        if entry.style != resolved {
            entry.style = resolved;
            self.invalidate(id);
        }
        Ok(())
    }

    pub(crate) fn set_text(
        &mut self,
        id: LocalId,
        text: Option<LayoutText>,
    ) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        let same = match (&entry.text, &text) {
            (Some(old), Some(new)) => old.same_source(new),
            (None, None) => true,
            _ => false,
        };
        entry.text = text;
        if !same {
            self.invalidate(id);
        }
        Ok(())
    }

    pub(crate) fn style_for(&self, id: LocalId) -> Option<&StyleData> {
        self.node(id).map(LayoutNode::style)
    }

    pub(crate) fn text_for(&self, id: LocalId) -> Option<&LayoutText> {
        self.node(id).and_then(LayoutNode::text)
    }

    pub(crate) fn node(&self, id: LocalId) -> Option<&LayoutNode> {
        let idx = id.index as usize;
        let entry = self.nodes.get(idx)?;
        if entry.alive && entry.tag == id.tag {
            Some(entry)
        } else {
            None
        }
    }

    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        let idx = id.index as usize;
        let entry = self.nodes.get(idx)?;
        if entry.alive && entry.tag == id.tag {
            self.boxes.get(idx).map(BoxCell::get)
        } else {
            None
        }
    }

    pub(crate) fn frame_for(&self, id: LocalId) -> Option<u32> {
        let idx = id.index as usize;
        let entry = self.nodes.get(idx)?;
        if entry.alive && entry.tag == id.tag {
            self.boxes.get(idx).map(BoxCell::frame)
        } else {
            None
        }
    }

    pub(crate) fn write(&mut self, id: LocalId, value: ComputedBox) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let frame = self.frame;
        let cell = self
            .boxes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        cell.write(value, frame);
        Ok(())
    }

    pub(crate) fn refer(&self, id: LocalId) -> Option<NodeRef<'_>> {
        self.node(id).map(|_| NodeRef { forest: self, id })
    }

    fn slot(&self, id: LocalId) -> Result<usize, LayoutError> {
        let idx = id.index as usize;
        let entry = self
            .nodes
            .get(idx)
            .filter(|n| n.alive && n.tag == id.tag)
            .ok_or_else(|| {
                LayoutError::LayoutInvalid(format!(
                    "stale layout-forest id index={} tag={}",
                    id.index, id.tag
                ))
            })?;
        let _ = entry;
        Ok(idx)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeRef<'a> {
    forest: &'a LayoutForest,
    id: LocalId,
}

impl NodeRef<'_> {
    pub(crate) const fn id(&self) -> LocalId {
        self.id
    }

    pub(crate) fn children(&self) -> &[LocalId] {
        self.forest
            .node(self.id)
            .map_or(&[], |entry| entry.children.as_slice())
    }

    pub(crate) fn kernel(&self) -> Option<NodeId> {
        self.forest.node(self.id).and_then(LayoutNode::kernel)
    }

    pub(crate) fn style(&self) -> &StyleData {
        self.forest
            .node(self.id)
            .map_or(&DEFAULT_STYLE, LayoutNode::style)
    }

    pub(crate) fn text(&self) -> Option<&LayoutText> {
        self.forest.node(self.id).and_then(LayoutNode::text)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "forest_tests.rs"]
mod tests;
