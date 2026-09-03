#![allow(clippy::redundant_pub_crate)]

use std::cell::Cell;
use std::hash::{Hash as _, Hasher as _};

use ahash::AHasher;
use sigil_kernel::{NodeId, TextContent};
use sigil_style::TextStyle;
use smallvec::SmallVec;

use crate::{LayoutError, StyleData, TextMeasure};

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

#[derive(Debug, Default)]
pub(crate) struct BoxCell {
    cb: Cell<ComputedBox>,
    frame: Cell<u32>,
}

impl BoxCell {
    #[must_use]
    pub(crate) const fn new(cb: ComputedBox) -> Self {
        Self {
            cb: Cell::new(cb),
            frame: Cell::new(0),
        }
    }

    pub(crate) const fn get(&self) -> ComputedBox {
        self.cb.get()
    }

    pub(crate) const fn frame(&self) -> u32 {
        self.frame.get()
    }

    pub(crate) fn write(&self, cb: ComputedBox, frame: u32) {
        self.cb.set(cb);
        self.frame.set(frame);
    }

    pub(crate) fn reset(&self) {
        self.cb.set(ComputedBox::default());
        self.frame.set(0);
    }
}

#[derive(Debug)]
pub(crate) struct LayoutForest {
    nodes: Vec<LayoutNode>,
    boxes: Vec<BoxCell>,
    free: Vec<LocalId>,
    frame: u32,
}

impl Default for LayoutForest {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutForest {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            boxes: Vec::new(),
            free: Vec::new(),
            frame: 1,
        }
    }

    pub(crate) const fn next_frame(&mut self) -> u32 {
        self.frame = self.frame.wrapping_add(1);
        self.frame
    }

    pub(crate) const fn current_frame(&self) -> u32 {
        self.frame
    }

    pub(crate) fn count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    pub(crate) fn insert(&mut self, kernel: Option<NodeId>, style: StyleData) -> LocalId {
        if let Some(slot) = self.free.pop() {
            let idx = slot.index as usize;
            let node = LayoutNode {
                kernel,
                style,
                text: None,
                children: SmallVec::new(),
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
            return slot;
        }
        let index = u32::try_from(self.nodes.len()).expect("forest slab exceeds u32::MAX");
        let id = LocalId { index, tag: 0 };
        self.nodes.push(LayoutNode {
            kernel,
            style,
            text: None,
            children: SmallVec::new(),
            tag: 0,
            alive: true,
        });
        self.boxes.push(BoxCell::new(ComputedBox::default()));
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
        entry.tag = next_tag;
        self.free.push(LocalId {
            index: id.index,
            tag: next_tag,
        });
        Ok(())
    }

    pub(crate) fn attach(&mut self, parent: LocalId, child: LocalId) -> Result<(), LayoutError> {
        let _ = self.slot(child)?;
        let slot = self.slot(parent)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the parent index is in-bounds");
        entry.children.push(child);
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
        Ok(())
    }

    pub(crate) fn set_style(&mut self, id: LocalId, style: StyleData) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        entry.style = style;
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
        entry.text = text;
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

static DEFAULT_STYLE: StyleData = StyleData {
    display: crate::style_data::Display::Block,
    position: crate::style_data::Position::Relative,
    overflow: crate::style_data::Size {
        width: crate::style_data::Overflow::Visible,
        height: crate::style_data::Overflow::Visible,
    },
    inset: crate::style_data::Rect {
        left: crate::style_data::LengthPercentageAuto::Auto,
        right: crate::style_data::LengthPercentageAuto::Auto,
        top: crate::style_data::LengthPercentageAuto::Auto,
        bottom: crate::style_data::LengthPercentageAuto::Auto,
    },
    flex_direction: crate::style_data::FlexDirection::Row,
    flex_wrap: crate::style_data::FlexWrap::NoWrap,
    flex_grow: 0.0,
    flex_shrink: 1.0,
    flex_basis: crate::style_data::Dimension::Auto,
    justify_content: None,
    align_items: None,
    align_content: None,
    align_self: None,
    gap: crate::style_data::Size {
        width: crate::style_data::LengthPercentage::Px(0.0),
        height: crate::style_data::LengthPercentage::Px(0.0),
    },
    margin: crate::style_data::Rect {
        left: crate::style_data::LengthPercentageAuto::Px(0.0),
        right: crate::style_data::LengthPercentageAuto::Px(0.0),
        top: crate::style_data::LengthPercentageAuto::Px(0.0),
        bottom: crate::style_data::LengthPercentageAuto::Px(0.0),
    },
    padding: crate::style_data::Rect {
        left: crate::style_data::LengthPercentage::Px(0.0),
        right: crate::style_data::LengthPercentage::Px(0.0),
        top: crate::style_data::LengthPercentage::Px(0.0),
        bottom: crate::style_data::LengthPercentage::Px(0.0),
    },
    border: crate::style_data::Rect {
        left: crate::style_data::LengthPercentage::Px(0.0),
        right: crate::style_data::LengthPercentage::Px(0.0),
        top: crate::style_data::LengthPercentage::Px(0.0),
        bottom: crate::style_data::LengthPercentage::Px(0.0),
    },
    size: crate::style_data::Size {
        width: crate::style_data::Dimension::Auto,
        height: crate::style_data::Dimension::Auto,
    },
    min_size: crate::style_data::Size {
        width: crate::style_data::Dimension::Auto,
        height: crate::style_data::Dimension::Auto,
    },
    max_size: crate::style_data::Size {
        width: crate::style_data::Dimension::Auto,
        height: crate::style_data::Dimension::Auto,
    },
    order: 0,
    box_sizing: crate::style_data::BoxSizing::ContentBox,
    grid_template_columns: Vec::new(),
    grid_template_rows: Vec::new(),
    grid_column_span: 1,
    grid_row_span: 1,
    grid_column_start: 0,
    grid_row_start: 0,
    grid_auto_flow_dense: false,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SpaceConstraint {
    Definite(f32),
    MinContent,
    MaxContent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AvailableSpace {
    pub(crate) w: SpaceConstraint,
    pub(crate) h: SpaceConstraint,
}

impl AvailableSpace {
    #[must_use]
    pub(crate) const fn definite(w: f32, h: f32) -> Self {
        Self {
            w: SpaceConstraint::Definite(w),
            h: SpaceConstraint::Definite(h),
        }
    }
}

pub(crate) struct MeasureScope<'a> {
    forest: &'a LayoutForest,
    text: &'a dyn TextMeasure,
    available: AvailableSpace,
}

impl<'a> MeasureScope<'a> {
    pub(crate) const fn new(
        forest: &'a LayoutForest,
        text: &'a dyn TextMeasure,
        available: AvailableSpace,
    ) -> Self {
        Self {
            forest,
            text,
            available,
        }
    }

    pub(crate) const fn forest(&self) -> &LayoutForest {
        self.forest
    }

    pub(crate) const fn text(&self) -> &dyn TextMeasure {
        self.text
    }

    pub(crate) const fn available(&self) -> AvailableSpace {
        self.available
    }

    pub(crate) fn write(&self, id: LocalId, value: ComputedBox) {
        let idx = id.index as usize;
        let Some(entry) = self.forest.nodes.get(idx) else {
            return;
        };
        if !entry.alive || entry.tag != id.tag {
            return;
        }
        if let Some(slot) = self.forest.boxes.get(idx) {
            slot.write(value, self.forest.current_frame());
        }
    }

    pub(crate) fn write_node(&self, node: &NodeRef<'_>, value: ComputedBox) {
        self.write(node.id(), value);
    }

    pub(crate) fn measure_text(
        &self,
        node: NodeRef<'_>,
        known: crate::style_data::Size<Option<f32>>,
        wrap_to: Option<f32>,
    ) -> Option<crate::style_data::Size<f32>> {
        let text = node.text()?;
        let memo_key = text_memo_key(node.id(), known, wrap_to, text);
        if let Some(hit) = text_memo_get(memo_key) {
            return Some(hit);
        }
        let width = if text.wrap {
            known.width.or(wrap_to).map_or_else(
                || space_constraint_to_hal_dim(self.available.w),
                sigil_hal::Dim::Fixed,
            )
        } else {
            sigil_hal::Dim::MaxContent
        };
        let space = sigil_hal::Size {
            width,
            height: space_constraint_to_hal_dim(self.available.h),
        };
        let measured = text.style.as_ref().map_or_else(
            || self.text.measure(&text.content, space),
            |style| self.text.measure_styled(&text.content, style, space),
        );
        let natural_w = if text.ellipsis {
            known
                .width
                .or(wrap_to)
                .map_or(measured.width, |box_w| measured.width.min(box_w))
        } else {
            measured.width
        };
        let out = crate::style_data::Size {
            width: known.width.unwrap_or(natural_w),
            height: known.height.unwrap_or(measured.height),
        };
        text_memo_store(memo_key, out);
        Some(out)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct TextMemoKey {
    index: u32,
    tag: u32,
    known_w: u32,
    known_h: u32,
    wrap: u32,
    content: u64,
}

fn text_memo_key(
    id: LocalId,
    known: crate::style_data::Size<Option<f32>>,
    wrap_to: Option<f32>,
    text: &LayoutText,
) -> TextMemoKey {
    TextMemoKey {
        index: id.slot(),
        tag: id.era(),
        known_w: known.width.map_or(u32::MAX, f32::to_bits),
        known_h: known.height.map_or(u32::MAX, f32::to_bits),
        wrap: wrap_to.map_or(u32::MAX, f32::to_bits),
        content: text_memo_content(text),
    }
}

fn text_memo_content(text: &LayoutText) -> u64 {
    let mut hasher = AHasher::default();
    match &text.content {
        TextContent::Static(value) => value.hash(&mut hasher),
        live => live.resolve().hash(&mut hasher),
    }
    text.wrap.hash(&mut hasher);
    text.ellipsis.hash(&mut hasher);
    if let Some(style) = &text.style {
        crate::text_style::text_style_signature(style).hash(&mut hasher);
    }
    hasher.finish()
}

const TEXT_MEMO_LIVE: usize = 8_192;

#[derive(Default)]
struct TextMemo {
    live: ahash::AHashMap<TextMemoKey, crate::style_data::Size<f32>>,
    stale: ahash::AHashMap<TextMemoKey, crate::style_data::Size<f32>>,
}

impl TextMemo {
    fn get(&mut self, key: TextMemoKey) -> Option<crate::style_data::Size<f32>> {
        if let Some(hit) = self.live.get(&key) {
            return Some(*hit);
        }
        let hit = self.stale.remove(&key)?;
        self.live.insert(key, hit);
        Some(hit)
    }

    fn store(&mut self, key: TextMemoKey, size: crate::style_data::Size<f32>) {
        if self.live.len() >= TEXT_MEMO_LIVE {
            self.stale = std::mem::take(&mut self.live);
        }
        self.live.insert(key, size);
    }

    fn clear(&mut self) {
        self.live.clear();
        self.stale.clear();
    }
}

fn text_memo_get(key: TextMemoKey) -> Option<crate::style_data::Size<f32>> {
    TEXT_MEMO.with(|cell| cell.borrow_mut().get(key))
}

fn text_memo_store(key: TextMemoKey, size: crate::style_data::Size<f32>) {
    TEXT_MEMO.with(|cell| cell.borrow_mut().store(key, size));
}

pub(crate) fn text_memo_clear() {
    TEXT_MEMO.with(|cell| cell.borrow_mut().clear());
}

thread_local! {
    static TEXT_MEMO: std::cell::RefCell<TextMemo> = std::cell::RefCell::new(TextMemo::default());
}

const fn space_constraint_to_hal_dim(space: SpaceConstraint) -> sigil_hal::Dim {
    match space {
        SpaceConstraint::Definite(value) => sigil_hal::Dim::Fixed(value),
        SpaceConstraint::MinContent => sigil_hal::Dim::MinContent,
        SpaceConstraint::MaxContent => sigil_hal::Dim::MaxContent,
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
