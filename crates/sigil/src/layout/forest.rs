#![allow(clippy::redundant_pub_crate)]

use crate::kernel::{NodeId, TextContent};
use crate::style::TextStyle;
use smallvec::SmallVec;

use crate::Error;
use crate::layout::StyleData;

mod box_cell;
mod intrinsic;
mod links;
mod measure;
mod solved;
mod text_memo;

pub(crate) use box_cell::BoxCell;
pub(crate) use intrinsic::IntrinsicKey;
use intrinsic::IntrinsicMemo;
pub(crate) use measure::{AvailableSpace, MeasureScope, SpaceConstraint};
pub(crate) use solved::SolvedKey;
use solved::SolvedSubtree;
use text_memo::TextMemo;
#[cfg(test)]
use text_memo::{TEXT_MEMO_LIVE, TextMemoKey};

static DEFAULT_STYLE: StyleData = StyleData::DEFAULT;

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
    fills: Vec<std::cell::Cell<u8>>,
    solved: Vec<std::cell::Cell<Option<SolvedSubtree>>>,
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
            fills: Vec::new(),
            solved: Vec::new(),
        }
    }

    pub(crate) fn next_frame(&mut self) -> u32 {
        self.frame = self.frame.wrapping_add(1);
        if self.frame == 0 {
            self.frame = 1;
            for cell in &self.boxes {
                cell.stamp(0);
            }
            self.clear_solved();
        }
        self.frame
    }

    pub(crate) const fn current_frame(&self) -> u32 {
        self.frame
    }

    #[cfg(feature = "kittest")]
    pub(crate) fn count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    pub(crate) fn insert(&mut self, kernel: Option<NodeId>, style: StyleData) -> LocalId {
        let slot = self.free.pop();
        let id = slot.unwrap_or_else(|| LocalId {
            index: u32::try_from(self.nodes.len()).expect("forest slab exceeds u32::MAX"),
            tag: 0,
        });
        let node = LayoutNode {
            kernel,
            style: crate::layout::units::resolve_style(style),
            text: None,
            children: SmallVec::new(),
            parent: None,
            tag: id.tag,
            alive: true,
        };
        let idx = id.index as usize;
        if let Some(entry) = slot.and_then(|_| self.nodes.get_mut(idx)) {
            *entry = node;
            if let Some(cell) = self.boxes.get(idx) {
                cell.reset();
            }
            self.forget(idx);
        } else {
            self.nodes.push(node);
            self.boxes.push(BoxCell::new(ComputedBox::default()));
            self.intrinsic.push(IntrinsicMemo::default());
            self.fills.push(std::cell::Cell::new(0));
            self.solved.push(std::cell::Cell::new(None));
        }
        id
    }

    pub(crate) fn set_style(&mut self, id: LocalId, style: StyleData) -> Result<(), Error> {
        let entry = self.live_mut(id)?;
        let resolved = crate::layout::units::resolve_style(style);
        if entry.style != resolved {
            entry.style = resolved;
            self.invalidate(id);
        }
        Ok(())
    }

    pub(crate) fn set_text(&mut self, id: LocalId, text: Option<LayoutText>) -> Result<(), Error> {
        let entry = self.live_mut(id)?;
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
        self.nodes
            .get(id.index as usize)
            .filter(|entry| entry.alive && entry.tag == id.tag)
    }

    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        self.node(id)
            .and_then(|_| self.boxes.get(id.index as usize))
            .map(BoxCell::get)
    }

    pub(crate) fn frame_for(&self, id: LocalId) -> Option<u32> {
        self.node(id)
            .and_then(|_| self.boxes.get(id.index as usize))
            .map(BoxCell::frame)
    }

    pub(crate) fn write(&mut self, id: LocalId, value: ComputedBox) -> Result<(), Error> {
        self.write_at_frame(id, value, self.frame)
    }

    pub(crate) fn write_at_frame(
        &mut self,
        id: LocalId,
        value: ComputedBox,
        frame: u32,
    ) -> Result<(), Error> {
        self.node(id).ok_or_else(|| stale(id))?;
        self.boxes
            .get_mut(id.index as usize)
            .ok_or_else(|| stale(id))?
            .write(value, frame);
        Ok(())
    }

    pub(crate) fn refer(&self, id: LocalId) -> Option<NodeRef<'_>> {
        self.node(id).map(|_| NodeRef { forest: self, id })
    }

    fn live_mut(&mut self, id: LocalId) -> Result<&mut LayoutNode, Error> {
        self.nodes
            .get_mut(id.index as usize)
            .filter(|entry| entry.alive && entry.tag == id.tag)
            .ok_or_else(|| stale(id))
    }
}

fn stale(id: LocalId) -> Error {
    Error::layout(format!(
        "stale layout-forest id index={} tag={}",
        id.index, id.tag
    ))
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
